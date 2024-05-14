{

  # Add a passthru derivation to a Rust derivation `package` that generates a
  # CycloneDX SBOM.
  #
  # This could be done much more elegantly if `buildRustPackage` supported
  # finalAttrs. When https://github.com/NixOS/nixpkgs/pull/194475 lands, we can
  # most likely get rid of this.
  rust = package: { cargo-cyclonedx }: package.overrideAttrs
    (previousAttrs: {
      passthru = (previousAttrs.passthru or { }) // {
        bombonVendoredSbom = package.overrideAttrs (previousAttrs: {
          nativeBuildInputs = (previousAttrs.nativeBuildInputs or [ ]) ++ [ cargo-cyclonedx ];
          outputs = [ "out" ];
          phases = [ "unpackPhase" "patchPhase" "buildPhase" "installPhase" ];
          buildPhase = ''
            cargo cyclonedx --spec-version 1.4 --format json
          '';
          installPhase = ''
            mkdir -p $out
            find . -name "*.cdx.json" -execdir install {} $out/{} \;
          '';
        });
      };
    });

}
