{ lib
, runCommand
, transformer
, buildtimeDependencies
, runtimeDependencies
}:

drv: { includeBuildtimeDependencies ? false }:
let
  flags = lib.optionals includeBuildtimeDependencies [
    "--include-buildtime-dependencies"
  ];
in
runCommand "${drv.name}.cdx.json" { buildInputs = [ transformer ]; } ''
  bombon-transformer ${drv} \
    ${toString flags} \
    ${buildtimeDependencies drv} \
    ${runtimeDependencies drv} > $out
''

