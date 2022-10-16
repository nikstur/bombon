let
  sources = import ./nix/sources.nix { };
  pkgs = import sources.nixpkgs { };
  bombon = import sources.bombon;
  system = "x86_64-linux";
in
{
  helloBom = bombon.lib.${system}.generateBom pkgs.hello;
}
