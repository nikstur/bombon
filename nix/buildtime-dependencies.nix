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

  # The store paths of the outputs that hold the libraries of `inputs`. Build
  # inputs are usually dev outputs.
  libraryPaths =
    inputs:
    lib.unique (
      map (input: (input.lib or input.out).outPath) (lib.filter lib.isDerivation (lib.flatten inputs))
    );

  # Like lib.getAttrs but omit attrs that do not exist.
  optionalGetAttrs =
    names: attrs: lib.genAttrs (builtins.filter (x: lib.hasAttr x attrs) names) (name: attrs.${name});

  # Retrieve only the required fields from a derivation.
  #
  # Also renames outPath so that builtins.toJSON actually emits JSON and not
  # only the nix store path.
  fields =
    drv:
    let
      # A derivation whose `meta.license` is a hand-rolled attrset or a plain string
      # does not carry `licenseType`, so `lib.licenses.toSPDX` throws
      # `attribute 'licenseType' missing` and aborts the entire SBOM. Guard the call.
      toSPDX =
        license:
        if license ? licenseType then
          lib.licenses.toSPDX license
        else if lib.isString license then
          license
        else
          "LicenseRef-unknown";

      isStatic = drv.stdenv.hostPlatform.isStatic or false;
      libc = drv.stdenv.cc.libc or null;
      propagated = libraryPaths (drv.propagatedBuildInputs or [ ]);

      meta = lib.recursiveUpdate drv.meta (
        lib.optionalAttrs (drv.meta ? license) {
          license = if !(lib.isList drv.meta.license) then toSPDX drv.meta.license else drv.meta.license;
        }
      );
    in
    (optionalGetAttrs [
      "name"
      "pname"
      "version"
      "outputName"
      "outputHash"
    ] drv)
    // {
      path = drv.outPath;
      patches = lib.flatten (drv.patches or [ ]);
      # The store paths of this derivation's direct build-time dependencies, so the transformer can emit build-time `dependsOn` edges.
      buildReferences = lib.unique (map (o: o.outPath) (lib.concatLists (drvDeps drv)));
    }
    # The libraries a static derivation links in, which leave no reference in
    # its output, so the runtime closure misses them. That includes the libc of
    # its compiler.
    // lib.optionalAttrs isStatic {
      hostInputs = lib.unique (
        libraryPaths (drv.buildInputs or [ ]) ++ propagated ++ lib.optional (libc != null) libc.outPath
      );
      # The inputs stdenv passes on to whatever builds against this derivation.
      propagatedHostInputs = propagated;
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
    }
    // lib.optionalAttrs (drv ? meta) {
      inherit meta;
    };

in

drv: extraPaths:

let

  allDrvs = [ drv ] ++ extraPaths;

  allBuildtimeDerivations = lib.flatten (map buildtimeDerivations allDrvs);

  allFields = map (item: fields item.drv) allBuildtimeDerivations;

  # The transformer reads only the patches and vendored SBOMs. The other store
  # paths are just names to it, and with their context building the SBOM
  # would realise the whole build closure.
  unformattedJson = writeText "${drv.name}-unformatted-buildtime-dependencies.json" (
    builtins.unsafeDiscardStringContext (builtins.toJSON allFields)
  );

  readPaths = lib.concatMap (
    f: f.patches ++ lib.optional (f ? vendoredSbom) f.vendoredSbom
  ) allFields;

in

# Format the json so that the transformer can better report where errors occur
runCommand "${drv.name}-buildtime-dependencies.json"
  {
    # There are too many paths for an environment variable.
    __structuredAttrs = true;
    inherit readPaths;
  }
  ''
    ${jq}/bin/jq < ${unformattedJson} > "$out"
  ''
