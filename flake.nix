{
  description = "Nix CycloneDX Software Bills of Materials (SBOMs)";

  inputs = {

    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    systems.url = "github:nix-systems/default";

    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };

    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    pre-commit-hooks-nix = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
        flake-compat.follows = "flake-compat";
      };
    };

    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

  };

  outputs = inputs@{ self, flake-parts, systems, ... }: flake-parts.lib.mkFlake { inherit inputs; } {
    systems = import systems;

    imports =
      let
        # This is effectively just boilerplate to allow us to keep the `lib`
        # output.
        libOutputModule = { lib, ... }: flake-parts.lib.mkTransposedPerSystemModule {
          name = "lib";
          option = lib.mkOption {
            type = lib.types.lazyAttrsOf lib.types.anything;
            default = { };
          };
          file = "";
        };
      in
      [
        inputs.pre-commit-hooks-nix.flakeModule
        libOutputModule
      ];

    flake = {
      templates.default = {
        path = builtins.filterSource (path: type: baseNameOf path == "flake.nix")
          ./examples/flakes;
        description = "Build a Bom for GNU hello";
      };
    };

    perSystem = { config, system, pkgs, lib, ... }:
      let
        # Include the Git commit hash as the version of bombon in generated Boms
        GIT_COMMIT = lib.optionalString (self ? rev) self.rev;

        transformer = pkgs.callPackage ./nix/packages/transformer.nix {
          inherit GIT_COMMIT;
        };

        buildBom = pkgs.callPackage ./nix/build-bom.nix {
          inherit transformer;
          buildtimeDependencies = pkgs.callPackage ./nix/buildtime-dependencies.nix { };
          runtimeDependencies = pkgs.callPackage ./nix/runtime-dependencies.nix { };
        };
      in
      {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [
            (_final: _prev: {
              inherit (inputs.gitignore.lib) gitignoreSource;
            })
          ];
        };

        lib = { inherit buildBom; };

        packages = {
          # This is mostly here for development
          inherit transformer;
          default = transformer;
        };

        checks = {
          clippy = transformer.overrideAttrs (_: previousAttrs: {
            nativeCheckInputs = (previousAttrs.nativeCheckInputs or [ ]) ++ [ pkgs.clippy ];
            checkPhase = "cargo clippy";
          });
          rustfmt = transformer.overrideAttrs (_: previousAttrs: {
            nativeCheckInputs = (previousAttrs.nativeCheckInputs or [ ]) ++ [ pkgs.rustfmt ];
            checkPhase = "cargo fmt --check";
          });
        } // import ./nix/tests { inherit pkgs buildBom; };

        pre-commit = {
          check.enable = true;

          settings = {
            hooks = {
              nixpkgs-fmt.enable = true;
              typos.enable = true;
            };

            settings.statix.ignore = [ "sources.nix" ];
          };
        };

        devShells.default = pkgs.mkShell {
          shellHook = ''
            ${config.pre-commit.installationScript}
          '';

          packages = [
            pkgs.clippy
            pkgs.rustfmt
          ];

          inputsFrom = [ transformer ];

          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

          inherit GIT_COMMIT;
        };

      };
  };
}
