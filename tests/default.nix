{ pkgs
, buildBom
}:

let
  testDerivations = with pkgs; [
    hello
    python3
    python3Packages.poetry # weird string license in buildtimeDependencies
  ];

  cycloneDxSpec = pkgs.fetchFromGitHub {
    owner = "CycloneDX";
    repo = "specification";
    rev = "1.3";
    sha256 = "sha256-nPQLvix401JTUAfw6f2nnvhQT8LxRGlsWkJ1Iq/26H0=";
  };

  # To avoid network access, the base URL is replaced with a local URI to the above downloaded schemas
  name = "bom-1.3.schema.json";
  relativeReferencesSchema = pkgs.runCommand name { } ''
    substitute "${cycloneDxSpec}/schema/${name}" "$out" \
      --replace 'http://cyclonedx.org/schema/${name}' 'file://${cycloneDxSpec}/schema/'
  '';

  buildBomAndValidate = drv:
    pkgs.runCommand "${drv.name}-bom-validation" { nativeBuildInputs = [ pkgs.check-jsonschema ]; } ''
      check-jsonschema \
        --schemafile "${relativeReferencesSchema}" \
        "${buildBom drv}" > $out
    '';

  genAttrsFromDrvs = drvs: f:
    builtins.listToAttrs (map (d: pkgs.lib.nameValuePair d.pname (f d)) drvs);
in
genAttrsFromDrvs testDerivations buildBomAndValidate
