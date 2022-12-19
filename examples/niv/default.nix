let
  sources = import ./nix/sources.nix { };
  pkgs = import sources.nixpkgs { };
  bombon = import sources.bombon;
  system = "x86_64-linux";
in
bombon.lib.${system}.buildBom pkgs.hello { }
