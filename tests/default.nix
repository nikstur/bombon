{ pkgs
, buildBom
}:

let
  buildtimeOptions = { includeBuildtimeDependencies = true; };

  # This list cannot grow indefinitely because building a Bom requires all
  # builtime dependenices to be downloaded or built. A lot of time is spent
  # evaluating, downloading, and building.
  testDerivations = with pkgs; [
    { name = "hello"; drv = hello; options = { }; }
    { name = "hello-buildtime"; drv = hello; options = buildtimeOptions; }

    { name = "python3"; drv = python3; options = { }; }
    { name = "python3-buildtime"; drv = python3; options = buildtimeOptions; }

    # weird string license in buildtimeDependencies
    { name = "poetry"; drv = python3Packages.poetry; options = { }; }
    { name = "poetry-buildtime"; drv = python3Packages.poetry; options = buildtimeOptions; }

    { name = "git"; drv = git; options = { }; }
    { name = "git-buildtime"; drv = git; options = buildtimeOptions; }
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

  buildBomAndValidate = drv: options:
    pkgs.runCommand "${drv.name}-bom-validation" { nativeBuildInputs = [ pkgs.check-jsonschema ]; } ''
      check-jsonschema \
        --schemafile "${relativeReferencesSchema}" \
        "${buildBom drv options}"
      touch $out
    '';

  genAttrsFromDrvs = drvs: f:
    builtins.listToAttrs (map (d: pkgs.lib.nameValuePair d.name (f d.drv d.options)) drvs);
in
genAttrsFromDrvs testDerivations buildBomAndValidate
