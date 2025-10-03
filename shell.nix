let
  sources = import ./lon.nix;
  pkgs = import sources.nixpkgs { };
in
pkgs.mkShell {
  packages = [
    pkgs.lon
    pkgs.clippy
    pkgs.rustfmt
    pkgs.cargo-machete
    pkgs.cargo-edit
    pkgs.cargo-bloat
    pkgs.cargo-deny
    pkgs.cargo-cyclonedx
  ];

  inputsFrom = [
    (pkgs.callPackage ./nix/packages/transformer.nix { })
  ];

  shellHook = ''
    ${(import ./nix/pre-commit.nix).shellHook}
  '';

  env = {
    RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
    LON_DIRECTORY = "nix";
  };
}
