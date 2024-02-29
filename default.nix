{ pkgs }: rec {

  transformer = pkgs.callPackage ./nix/packages/transformer.nix { };

  buildBom = pkgs.callPackage ./nix/build-bom.nix {
    inherit transformer;
    buildtimeDependencies = pkgs.callPackage ./nix/buildtime-dependencies.nix { };
    runtimeDependencies = pkgs.callPackage ./nix/runtime-dependencies.nix { };
  };

}
