{
  lib,
  rustPlatform,
}:

let
  cargoToml = builtins.fromTOML (builtins.readFile ../../rust/transformer/Cargo.toml);
in
rustPlatform.buildRustPackage rec {
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

  meta = with lib; {
    homepage = "https://github.com/nikstur/bombon";
    license = licenses.mit;
    maintainers = with lib.maintainers; [ nikstur ];
    mainProgram = "bombon-transformer";
    cpe = "cpe:2.3:a:nikstur:bombon-transformer:${version}:*:*:*:*:*:*:*";
  };
}
