{
  # Add a passthru derivation to a Rust derivation `package` that generates a
  # CycloneDX SBOM.
  rust =
    package:
    {
      pkgs,
      includeBuildtimeDependencies ? false,
    }:
    package.overrideAttrs (
      finalAttrs: previousAttrs: {
        passthru = (previousAttrs.passthru or { }) // {
          bombonVendoredSbom = finalAttrs.finalPackage.overrideAttrs (previousAttrs: {
            pname = previousAttrs.pname + "-bombon-vendored-sbom";
            nativeBuildInputs = (previousAttrs.nativeBuildInputs or [ ]) ++ [
              pkgs.buildPackages.cargo-cyclonedx
            ];
            outputs = [ "out" ];
            phases = [
              "unpackPhase"
              "patchPhase"
              "configurePhase"
              "buildPhase"
              "installPhase"
            ];

            buildPhase = ''
              if [ -n "''${cargoRoot-}" ]; then
                cd "$cargoRoot"
              fi

              cargo cyclonedx \
                --spec-version 1.5 \
                --format json \
                --describe binaries \
                --target ${finalAttrs.finalPackage.stdenv.hostPlatform.rust.rustcTarget} \
            ''
            + pkgs.lib.optionalString (previousAttrs.buildNoDefaultFeatures or false) " --no-default-features"
            + pkgs.lib.optionalString ((previousAttrs.buildFeatures or [ ]) != [ ]) (
              " --features " + builtins.concatStringsSep "," previousAttrs.buildFeatures
            )
            + pkgs.lib.optionalString (!includeBuildtimeDependencies) " --no-build-deps";

            installPhase = ''
              mkdir -p $out

              # Collect all paths to executable files. Cargo has no good support to find this
              # and this method is very robust. The flipside is that we have to build the package
              # to generate a BOM for it.
              #
              # Shared libraries (cdylib crate outputs) are executable too, but
              # `cargo cyclonedx --describe binaries` never emits an SBOM for them,
              # so requiring one below would fail any crate that builds a
              # `.so`/`.dylib`/`.dll` (e.g. FFI or PyO3 extension modules). Exclude
              # them from the scan.
              mapfile -d "" binaries < <(
                find ${finalAttrs.finalPackage} -type f ! -name ".*" \
                  ! -name "*.so" ! -name "*.so.*" ! -name "*.dylib" ! -name "*.dll" \
                  -executable -print0
              )

              for binary in "''${binaries[@]}"; do
                base=$(basename $binary)

                # Strip binary suffixes
                base=''${base%.exe}
                base=''${base%.efi}

                cdx=$(find . -name "''${base}_bin.cdx.json")

                if [ -f "$cdx" ]; then
                  echo "Found SBOM for binary '$binary': $cdx"
                  install -m444 "$cdx" $out/
                else
                  echo "Failed to find SBOM for binary: $binary"
                  exit 1
                fi
              done
            '';

            separateDebugInfo = false;

            # The SBOM derivation must not carry a vendored SBOM of its own.
            passthru = removeAttrs (previousAttrs.passthru or { }) [ "bombonVendoredSbom" ];
          });
        };
      }
    );

  # Add a passthru derivation that generates a CycloneDX SBOM to a derivation
  # `package` that vendors its npm dependencies with `fetchNpmDeps` and
  # installs them with `npmConfigHook` (e.g. anything built with
  # `buildNpmPackage`).
  #
  # `npmConfigHook` installs the vendored dependencies into `node_modules`, so
  # `npm sbom` can describe the exact dependency tree without any additional
  # tooling.
  npm =
    package:
    {
      pkgs,
      includeBuildtimeDependencies ? false,
    }:
    package.overrideAttrs (
      finalAttrs: previousAttrs: {
        passthru = (previousAttrs.passthru or { }) // {
          bombonVendoredSbom = finalAttrs.finalPackage.overrideAttrs (previousAttrs: {
            pname = previousAttrs.pname + "-bombon-vendored-sbom";
            outputs = [ "out" ];
            phases = [
              "unpackPhase"
              "patchPhase"
              "configurePhase"
              "buildPhase"
              "installPhase"
            ];

            buildPhase = ''
              if [ -n "''${npmRoot-}" ]; then
                cd "$npmRoot"
              fi

              npm sbom \
                --sbom-format cyclonedx \
                --sbom-type application \
                ''${npmWorkspace:+--workspace="$npmWorkspace"} \
            ''
            # Dev dependencies are only used to build the package, so they are
            # buildtime dependencies.
            + pkgs.lib.optionalString (!includeBuildtimeDependencies) " --omit dev"
            + " > vendored-sbom.cdx.json";

            installPhase = ''
              mkdir -p $out
              install -m444 vendored-sbom.cdx.json $out/${previousAttrs.pname}.cdx.json
            '';

            separateDebugInfo = false;

            # The SBOM derivation must not carry a vendored SBOM of its own.
            passthru = removeAttrs (previousAttrs.passthru or { }) [ "bombonVendoredSbom" ];
          });
        };
      }
    );

  # Add a passthru derivation that generates a CycloneDX SBOM to a derivation
  # `package` that vendors its pnpm dependencies with `fetchPnpmDeps` and
  # installs them with `pnpmConfigHook`.
  #
  # `pnpmConfigHook` installs the vendored dependencies into `node_modules`, so
  # `pnpm sbom` can describe the exact dependency tree without any additional
  # tooling. `pnpm sbom` is only available from pnpm 11 onwards.
  pnpm =
    package:
    {
      pkgs,
      includeBuildtimeDependencies ? false,
    }:
    package.overrideAttrs (
      finalAttrs: previousAttrs: {
        passthru = (previousAttrs.passthru or { }) // {
          bombonVendoredSbom = finalAttrs.finalPackage.overrideAttrs (previousAttrs: {
            pname = previousAttrs.pname + "-bombon-vendored-sbom";
            # `fetchPnpmDeps` is usually called with `finalAttrs.pname`, so the
            # renamed `pname` would otherwise produce a new fixed-output
            # derivation that fetches all dependencies again.
            inherit (finalAttrs.finalPackage) pnpmDeps;
            outputs = [ "out" ];
            phases = [
              "unpackPhase"
              "patchPhase"
              "configurePhase"
              "buildPhase"
              "installPhase"
            ];

            buildPhase = ''
              if [ -n "''${pnpmRoot-}" ]; then
                cd "$pnpmRoot"
              fi

              # Describe the same workspace packages that `pnpmConfigHook` installed.
              local -a pnpmSbomFlags
              concatTo pnpmSbomFlags pnpmWorkspaces
              pnpmSbomFlags=("''${pnpmSbomFlags[@]/#/--filter=}")

              pnpm sbom \
                --sbom-format cyclonedx \
                --sbom-spec-version 1.5 \
                --sbom-type application \
                "''${pnpmSbomFlags[@]}" \
            ''
            # Dev dependencies are only used to build the package, so they are
            # buildtime dependencies.
            + pkgs.lib.optionalString (!includeBuildtimeDependencies) " --prod"
            + " > vendored-sbom.cdx.json";

            installPhase = ''
              mkdir -p $out
              install -m444 vendored-sbom.cdx.json $out/${previousAttrs.pname}.cdx.json
            '';

            separateDebugInfo = false;

            # The SBOM derivation must not carry a vendored SBOM of its own.
            passthru = removeAttrs (previousAttrs.passthru or { }) [ "bombonVendoredSbom" ];
          });
        };
      }
    );

  # Add a passthru derivation that generates a CycloneDX SBOM to a derivation
  # `package` built with `buildGoModule`.
  #
  # `cyclonedx-gomod` reads the build info embedded in the compiled Go
  # binaries, so the SBOM describes exactly the modules linked into them.
  # Its license detection is disabled because it downloads the modules, which
  # does not work offline. Instead, `go-licenses.py` identifies the licenses
  # from the license files of the vendored modules with `licenseclassifier`.
  go =
    package:
    { pkgs }:
    package.overrideAttrs (
      finalAttrs: previousAttrs: {
        passthru = (previousAttrs.passthru or { }) // {
          bombonVendoredSbom = finalAttrs.finalPackage.overrideAttrs (previousAttrs: {
            pname = previousAttrs.pname + "-bombon-vendored-sbom";
            # `buildGoModule` names `goModules` after the `pname`, so the
            # renamed `pname` would otherwise produce a new fixed-output
            # derivation that fetches all modules again.
            inherit (finalAttrs.finalPackage) goModules;
            nativeBuildInputs = (previousAttrs.nativeBuildInputs or [ ]) ++ [
              pkgs.buildPackages.cyclonedx-gomod
              pkgs.buildPackages.licenseclassifier
              pkgs.buildPackages.python3
            ];
            outputs = [ "out" ];
            phases = [
              "unpackPhase"
              "patchPhase"
              "configurePhase"
              "buildPhase"
              "installPhase"
            ];

            buildPhase = ''
              for binary in ${finalAttrs.finalPackage}/bin/*; do
                # Skip files that are not Go binaries, e.g. wrapper scripts.
                go version -m "$binary" > /dev/null 2>&1 || continue

                cyclonedx-gomod bin \
                  -json \
                  -noserial \
                  -notimestamp \
                  -output-version 1.5 \
                  -output "$(basename "$binary").cdx.json" \
                  "$binary"
              done

              # The vendored modules: a vendor directory, or with `proxyVendor`
              # the module cache that `GOPROXY` points to.
              python3 ${./go-licenses.py} \
                ${pkgs.buildPackages.spdx-license-list-data.json}/json/licenses.json \
                ${if previousAttrs.proxyVendor or false then ''"$goModules"'' else "vendor"} \
                *.cdx.json
            '';

            installPhase = ''
              mkdir -p $out
              install -m444 *.cdx.json $out/
            '';

            separateDebugInfo = false;

            # The SBOM derivation must not carry a vendored SBOM of its own.
            passthru = removeAttrs (previousAttrs.passthru or { }) [ "bombonVendoredSbom" ];
          });
        };
      }
    );
}
