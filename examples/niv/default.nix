let
  sources = import ./nix/sources.nix { };
  pkgs = import sources.nixpkgs { };
  bombon = import sources.bombon { inherit pkgs; };
in
bombon.buildBom pkgs.hello { }
