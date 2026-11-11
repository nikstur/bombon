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

  # `age` with a `replace` directive that changes the path of a module. In a
  # vendor directory, the replacement is in the directory of the original
  # module. The upper case letters of the path are escaped in the module cache
  # of `proxyVendor`.
  ageReplace = pkgs.age.overrideAttrs (previousAttrs: {
    postPatch = (previousAttrs.postPatch or "") + ''
      cat >> go.mod <<EOF
      replace filippo.io/edwards25519 => github.com/FiloSottile/edwards25519 v1.1.0
      EOF
      cat >> go.sum <<EOF
      github.com/FiloSottile/edwards25519 v1.1.0 h1:S4P8Svv6l4z9f8MGxpO7un9oPaUAgPXXHgFTTCprCAU=
      github.com/FiloSottile/edwards25519 v1.1.0/go.mod h1:BxyFTGdWcka3PhytdK4V28tE5sGfRvvvRV7EaN4VDT4=
      EOF
    '';
    vendorHash = "sha256-JdBEnAdtyHc2pRNMU4uHhADvNCNlCUHOlqmO/zQtdqM=";
  });

  ageProxyVendor = ageReplace.overrideAttrs {
    proxyVendor = true;
    vendorHash = "sha256-66rzmefePCoFTkjmpb44aPeWxQPNUh3HVkHTLSm0Ngk=";
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
      name = "age-replace";
      drv = goPassthru ageReplace;
      options = { };
    }
    {
      name = "age-proxy-vendor";
      drv = goPassthru ageProxyVendor;
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

  genAttrsFromDrvs =
    drvs: f: builtins.listToAttrs (map (d: pkgs.lib.nameValuePair d.name (f d.drv d.options)) drvs);

  boms = genAttrsFromDrvs testDerivations buildBomAndValidate;
in
boms
// {
  # Each Go module gets the license that `licenseclassifier` identifies from
  # the vendored module, in the vendor directory and with `proxyVendor`.
  go-licenses = pkgs.runCommand "go-licenses" { nativeBuildInputs = [ pkgs.jq ]; } ''
    licenses() {
      jq -c --arg name "$2" '[.components[] | select(.name == $name) | .licenses]' "$1"
    }

    for sbom in ${boms.age} ${boms.age-replace} ${boms.age-proxy-vendor}; do
      [[ $(licenses "$sbom" golang.org/x/crypto) == '[[{"license":{"id":"BSD-3-Clause"}}]]' ]]
    done

    for sbom in ${boms.age-replace} ${boms.age-proxy-vendor}; do
      [[ $(licenses "$sbom" filippo.io/edwards25519) == '[]' ]]
      [[ $(licenses "$sbom" github.com/FiloSottile/edwards25519) == '[[{"license":{"id":"BSD-3-Clause"}}]]' ]]
    done

    touch $out
  '';

  # A license that is not on the SPDX list is a named license, or in an SPDX
  # expression a `LicenseRef-` with only the characters that SPDX allows.
  go-licenses-licenseref =
    pkgs.runCommand "go-licenses-licenseref"
      {
        nativeBuildInputs = [
          pkgs.jq
          pkgs.licenseclassifier
          pkgs.python3
        ];
      }
      ''
        assets=${pkgs.licenseclassifier.src}/v2/assets/License
        mkdir -p vendor/example.com/{one,two}
        cat > vendor/modules.txt <<EOF
        # example.com/one v1.0.0
        ## explicit
        # example.com/two v1.0.0
        ## explicit
        EOF
        cp $assets/Commons-Clause/license.txt vendor/example.com/one/LICENSE
        cp $assets/Khronos_OpenCL/pristine.txt vendor/example.com/two/LICENSE
        cp $assets/MIT/pristine.txt vendor/example.com/two/COPYING
        echo '{"components":[{"name":"example.com/one","version":"v1.0.0"},{"name":"example.com/two","version":"v1.0.0"}]}' > sbom.json

        python3 ${../go-licenses.py} \
          ${pkgs.spdx-license-list-data.json}/json/licenses.json vendor sbom.json

        [[ $(jq -c '.components[0].licenses' sbom.json) == '[{"license":{"name":"Commons-Clause"}}]' ]]
        [[ $(jq -c '.components[1].licenses' sbom.json) == '[{"expression":"LicenseRef-Khronos-OpenCL AND MIT"}]' ]]

        touch $out
      '';
}
