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
        packages.default = bombon.lib.${system}.buildBom pkgs.hello;
      });
}
