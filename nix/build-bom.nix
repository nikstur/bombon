{
  lib,
  runCommand,
  transformer,
  buildtimeDependencies,
  runtimeDependencies,
}:

drv:
{
  extraPaths ? [ ],
  metadataFrom ? [ ],
  includeBuildtimeDependencies ? false,
  excludes ? [ ],
}:

let
  args =
    lib.optionals includeBuildtimeDependencies [
      "--include-buildtime-dependencies"
    ]
    ++ lib.optionals (excludes != [ ]) (lib.map (e: "--exclude=${lib.escapeShellArg e}") excludes);
in
runCommand "${drv.name}.cdx.json"
  {
    __structuredAttrs = true;
    # SBOMs semantically don't have a closure.
    unsafeDiscardReferences.out = true;

    nativeBuildInputs = [
      transformer
    ];
  }
  ''
    bombon-transformer ${drv} \
      ${toString args} \
      --serial-number-seed "$out" \
      ${buildtimeDependencies drv extraPaths metadataFrom} \
      ${runtimeDependencies drv extraPaths} \
      "$out"
  ''
