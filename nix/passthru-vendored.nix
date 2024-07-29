{

  # Add a passthru derivation to a Rust derivation `package` that generates a
  # CycloneDX SBOM.
  #
  # This could be done much more elegantly if `buildRustPackage` supported
  # finalAttrs. When https://github.com/NixOS/nixpkgs/pull/194475 lands, we can
  # most likely get rid of this.
  rust = package: { pkgs }: package.overrideAttrs
    (previousAttrs: {
      passthru = (previousAttrs.passthru or { }) // {
        bombonVendoredSbom = package.overrideAttrs (previousAttrs: {
          pname = previousAttrs.pname + "-bombon-vendored-sbom";
          nativeBuildInputs = (previousAttrs.nativeBuildInputs or [ ]) ++ [ pkgs.cargo-cyclonedx ];
          outputs = [ "out" ];
          phases = [ "unpackPhase" "patchPhase" "configurePhase" "buildPhase" "installPhase" ];
          buildPhase = ''
            cargo cyclonedx --spec-version 1.4 --format json --describe binaries --target ${pkgs.stdenv.hostPlatform.rust.rustcTarget} \
          ''
          + pkgs.lib.optionalString
            (builtins.hasAttr "buildNoDefaultFeatures" previousAttrs && previousAttrs.buildNoDefaultFeatures)
            " --no-default-features"
          + pkgs.lib.optionalString
            (builtins.hasAttr "buildFeatures" previousAttrs && builtins.length previousAttrs.buildFeatures > 0)
            (" --features " + builtins.concatStringsSep "," previousAttrs.buildFeatures)
          ;
          installPhase = ''
            mkdir -p $out

            set +e
            installed_binaries=$(cargo build --bin 2>&1 | awk '/Available binaries:/ {found=1; next} found && NF {gsub(/[[:space:]]+/, ""); print}')
            set -e

            IFS=$'\n' binaries=($installed_binaries)
            for element in "''${binaries[@]}"
            do
                find . -name "''${element}_bin.cdx.json" -execdir install {} $out/{} \;
            done
          '';
          separateDebugInfo = false;
        });
      };
    });
}
