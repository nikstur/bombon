{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";
  };

  outputs = { self, nixpkgs, utils, naersk, pre-commit-hooks }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        transformer = naersk.lib.${system}.buildPackage ./transformer;
        generateBom = pkgs.callPackage ./bombon.nix { inherit transformer; };
      in
      {
        lib = { inherit generateBom; };

        packages = {
          # This is mostly here for development
          inherit transformer;
          default = transformer;
        };

        checks = {
          pre-commit = pre-commit-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              nixpkgs-fmt.enable = true;
              statix.enable = true;
              # Rustfmt and clippy are not included here because these hooks
              # don't work when the rust project is in a subdirectory
            };
          };
        };

        devShells.default = with pkgs; mkShell {
          buildInputs = [ cargo rustc rustfmt rustPackages.clippy ];
          inherit (self.checks.${system}.pre-commit) shellHook;
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };
      });
}
