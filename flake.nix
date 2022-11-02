{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
    pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";
  };

  outputs = { self, nixpkgs, utils, rust-overlay, crane, pre-commit-hooks }:
    let
      systems = nixpkgs.lib.remove "i686-linux" utils.lib.defaultSystems;
    in
    {
      templates.default = {
        path = builtins.filterSource (path: type: baseNameOf path == "flake.nix")
          ./examples/flakes;
        description = "Build a Bom for GNU hello";
      };
    } // utils.lib.eachSystem systems (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustVersion = "1.64.0";
        rustToolChain = pkgs.rust-bin.stable.${rustVersion}.default;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolChain;

        # Include the Git commit hash as the version of bombon in generated Boms
        GIT_COMMIT = pkgs.lib.optionalString (self ? rev) self.rev;

        transformer = craneLib.buildPackage {
          src = ./transformer;
          inherit GIT_COMMIT;
        };
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
        } // import ./tests { inherit pkgs buildBom; };

        devShells.default = with pkgs; mkShell {
          inputsFrom = [ transformer ];

          inherit GIT_COMMIT;

          inherit (self.checks.${system}.pre-commit) shellHook;
        };
      });
}
