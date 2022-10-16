{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    rust-overlay.url = "github:oxalica/rust-overlay";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";
  };

  outputs = { self, nixpkgs, utils, naersk, rust-overlay, pre-commit-hooks }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustVersion = "1.63.0";
        rustToolChain = pkgs.rust-bin.stable.${rustVersion}.default;
        naersk' = pkgs.callPackage naersk {
          cargo = rustToolChain;
          rustc = rustToolChain;
        };

        transformer = naersk'.buildPackage ./transformer;
        buildBom = pkgs.callPackage ./build-bom.nix {
          inherit transformer;
          buildtimeDependencies = pkgs.callPackage ./buildtime-dependencies.nix { };
          runtimeDependencies = pkgs.callPackage ./runtime-dependencies.nix { };
        };
      in
      {
        lib = { inherit buildBom; };

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
            settings = {
              statix.ignore = [ "sources.nix" ];
            };
          };
        };

        devShells.default = with pkgs; mkShell {
          inputsFrom = [ transformer ];

          inherit (self.checks.${system}.pre-commit) shellHook;
        };
      });
}
