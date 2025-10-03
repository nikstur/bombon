let
  sources = import ./lon.nix;
  pkgs = import sources.nixpkgs { };
  inherit (pkgs) lib;

  bombon = import ./. { inherit pkgs; };
  inherit (bombon) transformer buildBom passthruVendoredSbom;
in
{
  packages = {
    inherit transformer;
    sbom = buildBom transformer { };
  };

  checks = lib.recurseIntoAttrs {
    pre-commit = import ./nix/pre-commit.nix;

    transformer = lib.recurseIntoAttrs (
      {
        package = transformer;
      }
      // transformer.tests
    );

    integration = lib.recurseIntoAttrs (
      import ./nix/tests { inherit pkgs buildBom passthruVendoredSbom; }
    );
  };

}
