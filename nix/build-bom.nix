{
  lib,
  runCommand,
  transformer,
  cyclonedx-cli,
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
runCommand "${drv.name}.cdx.json"
  {
    nativeBuildInputs = [
      transformer
      cyclonedx-cli
    ];
  }
  ''
    bombon-transformer ${drv} \
      ${toString args} \
      ${buildtimeDependencies drv extraPaths} \
      ${runtimeDependencies drv extraPaths} \
      tmp.cdx.json

    cyclonedx convert \
      --input-format=json \
      --input-file=tmp.cdx.json \
      --output-version=v1_7 \
      --output-file=$out
  ''
