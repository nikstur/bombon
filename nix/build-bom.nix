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
  scanNodeModules ? false,
}:

let
  args =
    lib.optionals includeBuildtimeDependencies [
      "--include-buildtime-dependencies"
    ]
    ++ lib.optionals scanNodeModules [
      "--scan-node-modules"
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
      cyclonedx-cli
    ];
  }
  ''
    bombon-transformer ${drv} \
      ${toString args} \
      --serial-number-seed "$out" \
      ${buildtimeDependencies drv extraPaths} \
      ${runtimeDependencies drv extraPaths} \
      tmp.cdx.json

    cyclonedx convert \
      --input-format=json \
      --input-file=tmp.cdx.json \
      --output-version=v1_7 \
      --output-file=$out
  ''
