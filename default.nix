{ pkgs }:

let
  passthruVendoredSbom = import ./nix/passthru-vendored.nix;
  transformerWithoutSbom = pkgs.callPackage ./nix/packages/transformer.nix { };
in
rec {

  # It's useful to have these exposed for debugging. However, they are not a
  # public interface.
  __internal = {
    buildtimeDependencies = pkgs.callPackage ./nix/buildtime-dependencies.nix { };
    runtimeDependencies = pkgs.callPackage ./nix/runtime-dependencies.nix { };
  };

  transformer = passthruVendoredSbom.rust transformerWithoutSbom { inherit pkgs; };

  buildBom = pkgs.callPackage ./nix/build-bom.nix {
    inherit transformer;
    inherit (__internal) buildtimeDependencies runtimeDependencies;
  };

  inherit passthruVendoredSbom;

}
