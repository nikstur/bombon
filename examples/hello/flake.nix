{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    bombon.url = "github:nikstur/bombon";
  };

  outputs = { self, nixpkgs, utils, bombon }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        packages.helloBom = bombon.lib.${system}.generateBom pkgs.hello;
      });
}
