# This flake only serves as an entry point for users that want to use bombon
# via flakes. It is not used for development and thus does no expose anything
# useful for development.
{
  description = "Nix CycloneDX Software Bills of Materials (SBOMs)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, ... }:
    let
      eachSystem = nixpkgs.lib.genAttrs [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
    in
    {
      templates.default = {
        path = builtins.filterSource (path: _type: baseNameOf path == "flake.nix") ./examples/flakes;
        description = "Build a Bom for GNU hello";
      };

      lib = eachSystem (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          bombon = import ./. { inherit pkgs; };
        in
        {
          inherit (bombon) buildBom passthruVendoredSbom;
        }
      );
    };
}
