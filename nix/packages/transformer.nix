{
  lib,
  rustPlatform,
  clippy,
  rustfmt,
}:

let
  cargoToml = builtins.fromTOML (builtins.readFile ../../rust/transformer/Cargo.toml);
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = cargoToml.package.name;
  inherit (cargoToml.package) version;

  src = lib.sourceFilesBySuffices ../../rust/transformer [
    ".rs"
    ".toml"
    ".lock"
  ];

  cargoLock = {
    lockFile = ../../rust/transformer/Cargo.lock;
  };

  passthru.tests = {
    clippy = finalAttrs.finalPackage.overrideAttrs (
      _: previousAttrs: {
        pname = previousAttrs.pname + "-clippy";
        nativeCheckInputs = (previousAttrs.nativeCheckInputs or [ ]) ++ [ clippy ];
        checkPhase = "cargo clippy";
      }
    );
    rustfmt = finalAttrs.finalPackage.overrideAttrs (
      _: previousAttrs: {
        pname = previousAttrs.pname + "-rustfmt";
        nativeCheckInputs = (previousAttrs.nativeCheckInputs or [ ]) ++ [ rustfmt ];
        checkPhase = "cargo fmt --check";
      }
    );
  };

  meta = with lib; {
    homepage = "https://github.com/nikstur/bombon";
    license = licenses.mit;
    maintainers = with lib.maintainers; [ nikstur ];
    mainProgram = "bombon-transformer";
    identifiers.cpeParts = lib.meta.cpePatchVersionInUpdateWithVendor "nikstur" finalAttrs.version;
  };
})
