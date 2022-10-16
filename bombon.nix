{ runCommand
, closureInfo
, writeText
, lib
, transformer
}:
let
  # Returns a list of all of `drv0`'s inputs, a.k.a. buildtime dependencies.
  # Elements in the list has two fields:
  #
  #  * key: the store path of the input.
  #  * drv: the actual derivation object.
  #
  # There are no guarantees that this will find _all_ inputs, but it works well
  # enough in practice.
  # 
  # We include all the outputs because they each have different outPaths
  buildtimeDependencies = drv:
    let
      drvOutputs = drv:
        if builtins.hasAttr "outputs" drv
        then map (output: drv.${output}) drv.outputs
        else [ drv ];

      # Recurse into the derivation attributes to find new derivations
      drvDeps = lib.mapAttrsToList
        (k: v:
          if lib.isDerivation v then (drvOutputs v)
          else if lib.isList v
          then lib.concatMap drvOutputs (lib.filter lib.isDerivation v)
          else [ ]
        );

      wrap = drv: { key = drv.outPath; inherit drv; };

      # Walk through the whole DAG of dependencies, using the `outPath` as an
      # index for the elements.
      buildtimeDerivations = drv0: builtins.genericClosure {
        startSet = map wrap (drvOutputs drv0);
        operator = obj: map wrap (lib.concatLists (drvDeps obj.drv.drvAttrs));
      };

      # Works like lib.getAttrs but omits attrs that do not exist
      optionalGetAttrs = names: attrs:
        lib.genAttrs (builtins.filter (x: lib.hasAttr x attrs) names) (name: attrs.${name});

      # Retrieves only the required fields from a derivation and renames outPath so that 
      # builtins.toJSON actually emits JSON and not only the nix store path
      fields = drv:
        (optionalGetAttrs [ "name" "pname" "version" "meta" ] drv) // { path = drv.outPath; };
    in
    writeText "${drv.name}-buildtime-dependencies.json" (builtins.toJSON
      (map (obj: (fields obj.drv)) (buildtimeDerivations drv))
    );

  # This is a wrapper around nixpkgs' `closureInfo`. It produces a JSON file
  # containing a list of the store paths of `drv`'s runtime dependencies.
  runtimeDependencies = drv: runCommand "${drv.name}-runtime-dependencies.json" { } ''
    cat ${closureInfo { rootPaths = [ drv ]; }}/store-paths > $out
  '';
in
drv: runCommand "${drv.name}.cdx.json" { buildInputs = [ transformer ]; } ''
  bombon-transformer ${buildtimeDependencies drv} ${runtimeDependencies drv} > $out
''
