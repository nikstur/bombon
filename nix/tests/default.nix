{
  pkgs,
  buildBom,
  passthruVendoredSbom,
}:

let
  rustPassthru = pkg: passthruVendoredSbom.rust pkg { inherit pkgs; };
  npmPassthru = pkg: passthruVendoredSbom.npm pkg { inherit pkgs; };
  pnpmPassthru = pkg: passthruVendoredSbom.pnpm pkg { inherit pkgs; };
  goPassthru = pkg: passthruVendoredSbom.go pkg { inherit pkgs; };
  pyPassthru = pkg: passthruVendoredSbom.python pkg { inherit pkgs; };

  pythonScript =
    let
      pythonEnv = pkgs.python3.withPackages (ps: [ ps.pyyaml ]);
    in
    pkgs.writeShellApplication {
      name = "yaml-check";
      runtimeInputs = [ pythonEnv ];
      text = ''
        python -c "import yaml"
      '';
      passthru = { inherit pythonEnv; };
    };
  pythonScriptWithSbom = passthruVendoredSbom.python pythonScript {
    inherit pkgs;
    inherit (pythonScript) pythonEnv;
  };

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
    # A pattern with characters that are special to the shell and a leading dash
    {
      name = "hello-excludes";
      drv = hello;
      options = {
        excludes = [ "-(libunistring|libidn2)-[0-9.]*$" ];
      };
    }

    {
      name = "python3";
      drv = python3;
      options = { };
    }
    # Sacrificed to make space for the LLVM test
    # {
    #   name = "python3-buildtime";
    #   drv = python3;
    #   options = buildtimeOptions;
    # }

    # Takes too much storage for GitHub Actions
    # weird string license in buildtimeDependencies
    # {
    #   name = "poetry";
    #   drv = poetry;
    #   options = { };
    # }
    # Takes too much storage for GitHub Actions
    # {
    #   name = "poetry-buildtime";
    #   drv = poetry;
    #   options = buildtimeOptions;
    # }

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
        extraPaths = [ hello ];
      };
    }
    # Takes too much storage for GitHub Actions
    # {
    #   name = "git-extra-paths-buildtime";
    #   drv = git;
    #   options = buildtimeOptions // {
    #     extraPaths = [ poetry ];
    #   };
    # }

    {
      name = "cloud-hypervisor";
      drv = rustPassthru cloud-hypervisor;
      options = { };
    }
    {
      name = "cloud-hypervisor-buildtime";
      drv = rustPassthru cloud-hypervisor;
      options = buildtimeOptions;
    }

    {
      name = "pyright";
      drv = npmPassthru pyright;
      options = { };
    }
    {
      name = "pyright-buildtime";
      drv = npmPassthru pyright;
      options = buildtimeOptions;
    }

    {
      name = "taze";
      drv = pnpmPassthru taze;
      options = { };
    }
    {
      name = "taze-buildtime";
      drv = pnpmPassthru taze;
      options = buildtimeOptions;
    }

    {
      name = "lessc";
      drv = pnpmPassthru lessc;
      options = { };
    }

    {
      name = "age";
      drv = goPassthru age;
      options = { };
    }
    {
      name = "age-buildtime";
      drv = goPassthru age;
      options = buildtimeOptions;
    }

    {
      name = "yamllint";
      drv = pyPassthru yamllint;
      options = { };
    }
    {
      name = "requests";
      drv = pyPassthru python3Packages.requests;
      options = { };
    }
    {
      name = "python-script";
      drv = pythonScriptWithSbom;
      options = { };
    }

    # Multiple src urls
    {
      name = "kexec-tools";
      drv = kexec-tools;
      options = { };
    }

    # compound license
    {
      name = "llvm";
      drv = llvm;
      options = { };
    }
  ];

  cycloneDxVersion = "1.7";

  cycloneDxSpec = pkgs.fetchFromGitHub {
    owner = "CycloneDX";
    repo = "specification";
    # Potentially download a newer version than the one being checked because
    # it includes updated SPDX identifiers. They are stored in a file that just
    # lives alongside the CycloneDX schema file.
    rev = cycloneDxVersion;
    sha256 = "sha256-30u5dqNj3xgVO2MONdHJIoqwdgFSbyOwBQQc0AnoDWM=";
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

  # Only the inspected environment may end up in the SBOM, not the environment
  # cyclonedx-py itself runs in.
  checkVendoredPythonComponents =
    name: drv:
    pkgs.runCommand "${name}-vendored-components" { nativeBuildInputs = [ pkgs.jq ]; } ''
      sbom="${buildBom drv { }}"
      jq -e 'any(.components[]; .purl == "pkg:pypi/pyyaml@${pkgs.python3Packages.pyyaml.version}")' "$sbom"
      jq -e 'all(.components[]; .purl // "" | startswith("pkg:pypi/cyclonedx") | not)' "$sbom"
      touch "$out"
    '';

  genAttrsFromDrvs =
    drvs: f: builtins.listToAttrs (map (d: pkgs.lib.nameValuePair d.name (f d.drv d.options)) drvs);
in
genAttrsFromDrvs testDerivations buildBomAndValidate
//
  pkgs.lib.mapAttrs'
    (
      name: drv:
      pkgs.lib.nameValuePair "${name}-vendored-components" (checkVendoredPythonComponents name drv)
    )
    {
      yamllint = pyPassthru pkgs.yamllint;
      python-script = pythonScriptWithSbom;
    }
