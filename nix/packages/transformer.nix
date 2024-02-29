{ lib
, rustPlatform
}:

rustPlatform.buildRustPackage {
  pname = "bombon-transformer";
  version = (builtins.fromTOML (builtins.readFile ../../rust/transformer/Cargo.toml)).package.version;

  src = lib.sourceFilesBySuffices ../../rust/transformer [ ".rs" ".toml" ".lock" ];

  cargoLock = {
    lockFile = ../../rust/transformer/Cargo.lock;
  };

  meta = with lib; {
    homepage = "https://github.com/nikstur/bombon";
    license = licenses.mit;
    maintainers = with lib.maintainers; [ nikstur ];
    mainProgram = "bombon-transformer";
  };
}

