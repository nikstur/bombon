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
          nativeBuildInputs = (previousAttrs.nativeBuildInputs or [ ]) ++ [ pkgs.cargo-cyclonedx ];
          outputs = [ "out" ];
          phases = [ "unpackPhase" "patchPhase" "configurePhase" "buildPhase" "installPhase" ];
          buildPhase = ''
            cargo cyclonedx --spec-version 1.4 --format json --target ${pkgs.stdenv.hostPlatform.rust.rustcTarget}
          ''
          + pkgs.lib.optionalString
            (builtins.hasAttr "buildNoDefaultFeatures" previousAttrs)
            " --no-default-features"
          + pkgs.lib.optionalString
            (builtins.hasAttr "buildFeatures" previousAttrs)
            (" --features " + builtins.concatStringsSep "," previousAttrs.buildFeatures)
          ;
          installPhase = ''
            mkdir -p $out
            find . -name "*.cdx.json" -execdir install {} $out/{} \;
          '';
          separateDebugInfo = false;
        });
      };
    });
}
