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
  includeBuildtimeDependencies ? false,
  excludes ? [ ],
}:

let
  args =
    lib.optionals includeBuildtimeDependencies [
      "--include-buildtime-dependencies"
    ]
    ++ lib.optionals (excludes != [ ]) (lib.map (e: "--exclude=${lib.escapeShellArg e}") excludes)
    ++ lib.map (p: "--extra-path=${p}") extraPaths;
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
      ${buildtimeDependencies drv extraPaths} \
      ${runtimeDependencies drv extraPaths} \
      "$out"
  ''
