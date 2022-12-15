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

        rustToolChain = pkgs.rust-bin.fromRustupToolchainFile ./transformer/rust-toolchain.toml;
        craneLib = crane.lib.${system}.overrideToolchain rustToolChain;

        # Include the Git commit hash as the version of bombon in generated Boms
        GIT_COMMIT = pkgs.lib.optionalString (self ? rev) self.rev;

        commonArgs = {
          src = craneLib.cleanCargoSource ./transformer;
          inherit GIT_COMMIT;
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        transformer = craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });

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
          clippy = craneLib.cargoClippy (commonArgs // { inherit cargoArtifacts; });
          rustfmt = craneLib.cargoFmt (commonArgs // { inherit cargoArtifacts; });
          pre-commit = pre-commit-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              nixpkgs-fmt.enable = true;
              statix.enable = true;
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
