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
}
