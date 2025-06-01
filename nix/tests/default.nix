{
  pkgs,
  buildBom,
  passthruVendoredSbom,
}:

let
  rustPassthru = pkg: passthruVendoredSbom.rust pkg { inherit pkgs; };

  buildtimeOptions = {
    includeBuildtimeDependencies = true;
  };

  # This list cannot grow indefinitely because building a Bom requires all
  # builtime dependencies to be downloaded or built. A lot of time is spent
  # evaluating, downloading, and building.
  testDerivations = with pkgs; [
    {
      name = "hello";
      drv = hello;
      options = { };
    }
    {
      name = "hello-buildtime";
      drv = hello;
      options = buildtimeOptions;
    }

    {
      name = "python3";
      drv = python3;
      options = { };
    }
    {
      name = "python3-buildtime";
      drv = python3;
      options = buildtimeOptions;
    }

    # weird string license in buildtimeDependencies
    {
      name = "poetry";
      drv = poetry;
      options = { };
    }
    {
      name = "poetry-buildtime";
      drv = poetry;
      options = buildtimeOptions;
    }

    {
      name = "git";
      drv = git;
      options = { };
    }
    {
      name = "git-buildtime";
      drv = git;
      options = buildtimeOptions;
    }

    {
      name = "git-extra-paths";
      drv = git;
      options = {
        extraPaths = [ poetry ];
      };
    }
    {
      name = "git-extra-paths-buildtime";
      drv = git;
      options = buildtimeOptions // {
        extraPaths = [ poetry ];
      };
    }

    {
      name = "cloud-hypervisor";
      drv = rustPassthru cloud-hypervisor;
      options = { };
    }
    {
      name = "cloud-hypervisor";
      drv = rustPassthru cloud-hypervisor;
      options = buildtimeOptions;
    }
  ];

  cycloneDxVersion = "1.5";

  cycloneDxSpec = pkgs.fetchFromGitHub {
    owner = "CycloneDX";
    repo = "specification";
    rev = cycloneDxVersion;
    sha256 = "sha256-bAXi7m7kWJ+lZNYnvSNmLQ6+kLqRw379crbC3viNqzY=";
  };

  buildBomAndValidate =
    drv: options:
    pkgs.runCommand "${drv.name}-bom-validation" { nativeBuildInputs = [ pkgs.check-jsonschema ]; } ''
      sbom="${buildBom drv options}"
      check-jsonschema \
        --schemafile "${cycloneDxSpec}/schema/bom-${cycloneDxVersion}.schema.json" \
        --base-uri "${cycloneDxSpec}/schema/bom-${cycloneDxVersion}.schema.json" \
        "$sbom"
      ln -s $sbom $out
    '';

  genAttrsFromDrvs =
    drvs: f: builtins.listToAttrs (map (d: pkgs.lib.nameValuePair d.name (f d.drv d.options)) drvs);
in
genAttrsFromDrvs testDerivations buildBomAndValidate
