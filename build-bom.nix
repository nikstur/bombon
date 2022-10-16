{ runCommand
, transformer
, buildtimeDependencies
, runtimeDependencies
}:

drv:
runCommand "${drv.name}.cdx.json" { buildInputs = [ transformer ]; } ''
  bombon-transformer ${buildtimeDependencies drv} ${runtimeDependencies drv} > $out
''
