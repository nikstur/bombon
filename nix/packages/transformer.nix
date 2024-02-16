{ lib
, rustPlatform
, gitignoreSource

, GIT_COMMIT
}:

rustPlatform.buildRustPackage {
  pname = "bombon-transformer";
  version = "0.1.0";

  src = gitignoreSource ../../rust/transformer;

  cargoLock = {
    lockFile = ../../rust/transformer/Cargo.lock;
  };

  env = {
    inherit GIT_COMMIT;
  };

  meta = with lib; {
    homepage = "https://github.com/nikstur/bombon";
    license = licenses.mit;
    maintainers = with lib.maintainers; [ nikstur ];
    mainProgram = "bombon-transformer";
  };
}

