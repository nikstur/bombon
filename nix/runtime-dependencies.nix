# This returns a list of the store paths of drv's runtime dependencies.
# The additionally emitted `references` are the direct runtime references of `path`,
# needed to emit a CycloneDX `dependencies` graph.
{
  runCommand,
  jq,
}:

drv: extraPaths:
runCommand "${drv.name}-runtime-dependencies.json"
  {
    __structuredAttrs = true;
    exportReferencesGraph.closure = [ drv ] ++ extraPaths;
    nativeBuildInputs = [ jq ];
  }
  ''
    jq '[.closure[] | { path: .path, references: .references }]' \
      "$NIX_ATTRS_JSON_FILE" > "$out"
  ''
