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
    ++ lib.optionals (excludes != [ ]) (lib.map (e: "--exclude ${e}") excludes);
in
runCommand "${drv.name}.cdx.json" { nativeBuildInputs = [ transformer ]; } ''
  bombon-transformer ${drv} \
    ${toString args} \
    ${buildtimeDependencies drv extraPaths} \
    ${runtimeDependencies drv extraPaths} \
    $out
''
