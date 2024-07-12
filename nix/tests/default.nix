{ pkgs
, buildBom
, passthruVendoredSbom
}:

let
  rustPassthru = pkg: pkgs.callPackage (passthruVendoredSbom.rust pkg) { };

  buildtimeOptions = { includeBuildtimeDependencies = true; };

  # This list cannot grow indefinitely because building a Bom requires all
  # builtime dependencies to be downloaded or built. A lot of time is spent
  # evaluating, downloading, and building.
  testDerivations = with pkgs; [
    { name = "hello"; drv = hello; options = { }; }
    { name = "hello-buildtime"; drv = hello; options = buildtimeOptions; }

    { name = "python3"; drv = python3; options = { }; }
    { name = "python3-buildtime"; drv = python3; options = buildtimeOptions; }

    # weird string license in buildtimeDependencies
    { name = "poetry"; drv = poetry; options = { }; }
    { name = "poetry-buildtime"; drv = poetry; options = buildtimeOptions; }

    { name = "git"; drv = git; options = { }; }
    { name = "git-buildtime"; drv = git; options = buildtimeOptions; }

    { name = "git-extra-paths"; drv = git; options = { extraPaths = [ poetry ]; }; }
    { name = "git-extra-paths-buildtime"; drv = git; options = buildtimeOptions // { extraPaths = [ poetry ]; }; }

    { name = "cloud-hypervisor"; drv = rustPassthru cloud-hypervisor; options = { }; }
    { name = "cloud-hypervisor"; drv = rustPassthru cloud-hypervisor; options = buildtimeOptions; }
  ];

  cycloneDxSpec = pkgs.fetchFromGitHub {
    owner = "CycloneDX";
    repo = "specification";
    rev = "1.4";
    sha256 = "sha256-N9aEK2oYk3SoCczrRMt5ycdgXCPA5SHTKsS2CffFY14=";
  };

  buildBomAndValidate = drv: options:
    pkgs.runCommand "${drv.name}-bom-validation" { nativeBuildInputs = [ pkgs.check-jsonschema ]; } ''
      check-jsonschema \
        --schemafile "${cycloneDxSpec}/schema/bom-1.4.schema.json" \
        --base-uri "${cycloneDxSpec}/schema/bom-1.4.schema.json" \
        "${buildBom drv options}"
      touch $out
    '';

  genAttrsFromDrvs = drvs: f:
    builtins.listToAttrs (map (d: pkgs.lib.nameValuePair d.name (f d.drv d.options)) drvs);
in
genAttrsFromDrvs testDerivations buildBomAndValidate
