{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, naersk }:
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

        devShells.default = with pkgs; mkShell {
          buildInputs = [ cargo rustc rustfmt rustPackages.clippy ];
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };
      });
}
