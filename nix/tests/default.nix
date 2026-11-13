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

    # A store that is not writable, like a store used from the Nix store
    {
      name = "taze-read-only-store";
      drv = pnpmPassthru (
        taze.overrideAttrs {
          preConfigure = ''
            makeStoreReadOnly() {
              chmod -R a-w "$(pnpm config get store-dir)"
            }
            postConfigureHooks+=(makeStoreReadOnly)
          '';
        }
      );
      options = { };
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

  # Check the SBOM of `drv` with a jq expression that must evaluate to true.
  buildBomAndCheck =
    name: drv: options: check:
    pkgs.runCommand "${name}-bom-check" { nativeBuildInputs = [ pkgs.jq ]; } ''
      jq --exit-status ${pkgs.lib.escapeShellArg check} "${buildBom drv options}"
      touch $out
    '';

  checks = {
    # Only the `bin` output is referenced directly, but it is merged into
    # `out`, so the edge to it must be redirected.
    openssl-outputs =
      buildBomAndCheck "openssl-outputs"
        (pkgs.runCommand "openssl-outputs" { inherit (pkgs) openssl; } ''
          mkdir $out
          ln -s ${pkgs.openssl.bin} $out/bin
        '')
        { }
        ''
          .metadata.component."bom-ref" as $root
          | [.components[] | select(.name == "openssl")] as $openssl
          | ($openssl | length) == 1
            and ($openssl[0]."bom-ref" | endswith("-bin") | not)
            and (.dependencies[] | select(.ref == $root) | .dependsOn | index($openssl[0]."bom-ref"))
        '';

    # Two derivations with the same name and version are one component, which
    # depends on what either of them depends on.
    namesakes =
      let
        twin =
          dependency:
          pkgs.stdenvNoCC.mkDerivation {
            pname = "twin";
            version = "1.0";
            buildCommand = "ln -s ${dependency} $out";
          };
      in
      buildBomAndCheck "namesakes"
        (pkgs.runCommand "namesakes" { } ''
          mkdir $out
          ln -s ${twin pkgs.hello} $out/a
          ln -s ${twin pkgs.zlib} $out/b
        '')
        { }
        ''
          [.components[] | select(.name == "twin") | ."bom-ref"] as $twin
          | [.components[] | select(.name == "hello" or .name == "zlib") | ."bom-ref"] as $dependencies
          | ($twin | length) == 1
            and ($dependencies | length) == 2
            and (.dependencies[] | select(.ref == $twin[0]) | .dependsOn | contains($dependencies))
        '';

    # Only the `lib` output is shipped, so it is kept over the build-time `out`.
    runtime-output =
      let
        multi = pkgs.stdenvNoCC.mkDerivation {
          pname = "multi";
          version = "1.0";
          outputs = [
            "out"
            "lib"
          ];
          buildCommand = ''
            echo out > $out
            echo lib > $lib
          '';
        };
      in
      buildBomAndCheck "runtime-output"
        (pkgs.runCommand "runtime-output" { multiLib = multi.lib; } ''
          ln -s $multiLib $out
        '')
        buildtimeOptions
        ''
          [.components[] | select(.name == "multi") | ."bom-ref"] as $multi
          | ($multi | length) == 1 and ($multi[0] | endswith("-lib"))
        '';

    # Only build inputs: the outputs of `multi` and the two derivations named
    # `twin-1.0` each become one component, which keeps all their dependencies.
    buildtime-outputs-and-namesakes =
      let
        leaf =
          pname:
          pkgs.stdenvNoCC.mkDerivation {
            inherit pname;
            version = "1.0";
            buildCommand = "touch $out";
          };
        multi = pkgs.stdenvNoCC.mkDerivation {
          pname = "multi";
          version = "1.0";
          outputs = [
            "out"
            "dev"
          ];
          buildCommand = ''
            echo out > $out
            echo dev > $dev
          '';
        };
        twin =
          dependency:
          pkgs.stdenvNoCC.mkDerivation {
            pname = "twin";
            version = "1.0";
            inherit dependency;
            buildCommand = "touch $out";
          };
      in
      buildBomAndCheck "buildtime-outputs-and-namesakes"
        (pkgs.runCommand "buildtime-outputs-and-namesakes" {
          multiDev = multi.dev;
          twinA = twin (leaf "leaf-a");
          twinB = twin (leaf "leaf-b");
        } "touch $out")
        buildtimeOptions
        ''
          .metadata.component."bom-ref" as $root
          | (.dependencies[] | select(.ref == $root) | .dependsOn) as $direct
          | [.components[] | select(.name == "multi") | ."bom-ref"] as $multi
          | [.components[] | select(.name == "twin") | ."bom-ref"] as $twin
          | [.components[] | select(.name | startswith("leaf-")) | ."bom-ref"] as $leaves
          | ($multi | length) == 1
            and ($multi[0] | endswith("-dev") | not)
            and ($twin | length) == 1
            and ($leaves | length) == 2
            and ($direct | contains($multi + $twin))
            and (.dependencies[] | select(.ref == $twin[0]) | .dependsOn | contains($leaves))
        '';

    # An image doesn't reference its contents; they are passed as extraPaths.
    image =
      buildBomAndCheck "image"
        (pkgs.runCommand "image"
          {
            __structuredAttrs = true;
            unsafeDiscardReferences.out = true;
          }
          ''
            mkdir $out
            echo ${pkgs.hello} > $out/contents
          ''
        )
        { extraPaths = [ pkgs.hello ]; }
        ''
          .metadata.component."bom-ref" as $root
          | [.components[] | select(.name == "hello") | ."bom-ref"] as $hello
          | ($hello | length) == 1
            and (.dependencies[] | select(.ref == $root) | .dependsOn | index($hello[0]))
        '';

    # The script has no version, so it is not a component, but what it runs
    # must still be connected to the root.
    versionless-script =
      buildBomAndCheck "versionless-script"
        (pkgs.runCommand "versionless-script"
          {
            script = pkgs.writeShellScript "run-hello" "exec ${pkgs.hello}/bin/hello";
          }
          ''
            mkdir $out
            ln -s $script $out/run-hello
          ''
        )
        { }
        ''
          .metadata.component."bom-ref" as $root
          | [.components[] | select(.name == "hello") | ."bom-ref"] as $hello
          | ($hello | length) == 1
            and (.components | map(.name) | index("run-hello") | not)
            and (.dependencies[] | select(.ref == $root) | .dependsOn | index($hello[0]))
        '';
  };
in
genAttrsFromDrvs testDerivations buildBomAndValidate // checks
