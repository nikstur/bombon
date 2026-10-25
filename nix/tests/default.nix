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

  buildtimeOptions = {
    includeBuildtimeDependencies = true;
  };

  # A package that installs npm packages into a `node_modules` directory
  # without a vendored SBOM, as e.g. packages built with yarn do.
  nodeModules = pkgs.stdenvNoCC.mkDerivation {
    pname = "node-modules";
    version = "1.0.0";
    dontUnpack = true;
    installPhase = ''
      package() {
        mkdir -p "$out/lib/node_modules/$1"
        echo "$2" > "$out/lib/node_modules/$1/package.json"
      }
      package app '{"name": "app", "version": "1.0.0", "license": "MIT"}'
      package app/node_modules/@scope/a '{"name": "@scope/a", "version": "2.0.0", "license": "(MIT OR Apache-2.0)"}'
      package app/node_modules/b '{"name": "b", "version": "3.0.0", "license": "SEE LICENSE IN LICENSE"}'
      # Not a package because it is not directly inside a `node_modules` directory.
      package app/dist '{"name": "dist", "version": "1.0.0"}'
    '';
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
      name = "node-modules";
      drv = nodeModules;
      options = {
        scanNodeModules = true;
      };
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
    # The packages that the vendored SBOM already describes are not added again.
    {
      name = "lessc-node-modules";
      drv = pnpmPassthru lessc;
      options = {
        scanNodeModules = true;
      };
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

  genAttrsFromDrvs =
    drvs: f: builtins.listToAttrs (map (d: pkgs.lib.nameValuePair d.name (f d.drv d.options)) drvs);

  # Check that the npm packages of `nodeModules` are components with their
  # licenses and dependencies of the store path that contains them.
  nodeModulesContents =
    pkgs.runCommand "node-modules-bom-contents" { nativeBuildInputs = [ pkgs.jq ]; }
      ''
        jq --exit-status '
          [.components[] | select(.purl | startswith("pkg:npm/"))] as $npm
          | ($npm | map({ (.purl): .licenses }) | add) == {
              "pkg:npm/app@1.0.0": [{ "license": { "id": "MIT" } }],
              "pkg:npm/%40scope/a@2.0.0": [{ "expression": "(MIT OR Apache-2.0)" }],
              "pkg:npm/b@3.0.0": [{ "license": { "name": "SEE LICENSE IN LICENSE" } }]
            }
          and ($npm | all(."bom-ref" == .purl))
          and (
            [.dependencies[] | select(.ref | endswith("-node-modules-1.0.0")) | .dependsOn[]]
            | sort == ($npm | map(.purl) | sort)
          )
        ' ${buildBom nodeModules { scanNodeModules = true; }}
        touch $out
      '';
in
genAttrsFromDrvs testDerivations buildBomAndValidate
// {
  node-modules-contents = nodeModulesContents;
}
