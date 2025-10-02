{
  lib,
  writeText,
  runCommand,
  jq,
}:

let

  # Find the outputs of a derivation.
  #
  # Returns a list of all derivations that correspond to an output of the input
  # derivation.
  drvOutputs =
    drv: if builtins.hasAttr "outputs" drv then map (output: drv.${output}) drv.outputs else [ drv ];

  # Find the dependencies of a derivation via it's `drvAttrs`.
  #
  # Returns a list of all dependencies.
  drvDeps =
    drv:
    lib.mapAttrsToList (
      _k: v:
      if lib.isDerivation v then
        (drvOutputs v)
      else if lib.isList v then
        lib.concatMap drvOutputs (lib.filter lib.isDerivation v)
      else
        [ ]
    ) drv.drvAttrs;

  wrap = drv: {
    key = drv.outPath;
    inherit drv;
  };

  # Walk through the whole DAG of dependencies, using the `outPath` as an
  # index for the elements.
  #
  # Returns a list of all of `drv`'s buildtime dependencies.
  # Elements in the list have two fields:
  #
  #  - key: the store path of the input.
  #  - drv: the actual derivation object.
  #
  # All outputs are included because they have different outPaths
  buildtimeDerivations =
    drv0:
    builtins.genericClosure {
      startSet = map wrap (drvOutputs drv0);
      operator = item: map wrap (lib.concatLists (drvDeps item.drv));
    };

  # Like lib.getAttrs but omit attrs that do not exist.
  optionalGetAttrs =
    names: attrs: lib.genAttrs (builtins.filter (x: lib.hasAttr x attrs) names) (name: attrs.${name});

  # Retrieve only the required fields from a derivation.
  #
  # Also renames outPath so that builtins.toJSON actually emits JSON and not
  # only the nix store path.
  fields =
    drv:
    (optionalGetAttrs [
      "name"
      "pname"
      "version"
      "meta"
      "outputName"
      "outputHash"
    ] drv)
    // {
      path = drv.outPath;
      patches = lib.flatten (drv.patches or [ ]);
    }
    // lib.optionalAttrs (drv ? src && drv.src ? urls) {
      src = {
        inherit (drv.src) urls;
      }
      // lib.optionalAttrs (drv.src ? outputHash) {
        hash = drv.src.outputHash;
      };
    }
    // lib.optionalAttrs (drv ? bombonVendoredSbom) {
      vendoredSbom = drv.bombonVendoredSbom.outPath;
    };

in

drv: extraPaths:

let

  allDrvs = [ drv ] ++ extraPaths;

  allBuildtimeDerivations = lib.flatten (map buildtimeDerivations allDrvs);

  unformattedJson = writeText "${drv.name}-unformatted-buildtime-dependencies.json" (
    builtins.toJSON (map (item: (fields item.drv)) allBuildtimeDerivations)
  );

in

# Format the json so that the transformer can better report where errors occur
runCommand "${drv.name}-buildtime-dependencies.json" { } ''
  ${jq}/bin/jq < ${unformattedJson} > "$out"
''
