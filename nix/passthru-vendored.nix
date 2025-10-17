{
  # use like this:
  # example = bombon.passthruVendoredSbom.rustPackage { inherit pkgs; } {
  #   pname = "example-rust";
  #   version = "0.1.0";
  #   src = lib.cleanSourceWith {
  #     filter = customFilter ../.;
  #     src = ../.;
  #   };
  #   cargoLock.lockFile = ../Cargo.lock;
  # };
  rustPackage =
    {
      pkgs,
      includeBuildtimeDependencies ? false,
    }:
    attrs:
    pkgs.rustPlatform.buildRustPackage (
      finalAttrs:
      attrs
      // {
        passthru.bombonVendoredSbom = pkgs.rustPlatform.buildRustPackage {
          pname = finalAttrs.pname + "-bombon-vendored-sbom";
          inherit (finalAttrs) version cargoDeps src;

          nativeBuildInputs = (finalAttrs.nativeBuildInputs or [ ]) ++ [
            pkgs.buildPackages.cargo-cyclonedx
          ];
          outputs = [ "out" ];
          phases = [
            "unpackPhase"
            "patchPhase"
            "configurePhase"
            "buildPhase"
            "installPhase"
          ];

          buildPhase =
            ''
              cargo cyclonedx \
                --spec-version 1.5 \
                --format json \
                --describe binaries \
                --target ${pkgs.stdenv.hostPlatform.rust.rustcTarget} \
            ''
            + pkgs.lib.optionalString (
              builtins.hasAttr "buildNoDefaultFeatures" finalAttrs && finalAttrs.buildNoDefaultFeatures
            ) " --no-default-features"
            + pkgs.lib.optionalString (
              builtins.hasAttr "buildFeatures" finalAttrs && builtins.length finalAttrs.buildFeatures > 0
            ) (" --features " + builtins.concatStringsSep "," finalAttrs.buildFeatures)
            + pkgs.lib.optionalString (!includeBuildtimeDependencies) " --no-build-deps";

          installPhase = ''
            mkdir -p $out
            # Collect all paths to executable files. Cargo has no good support to find this
            # and this method is very robust. The flipside is that we have to build the package
            # to generate a BOM for it.
            mapfile -d "" binaries < <(find ${finalAttrs.finalPackage} -type f -executable -print0)

            for binary in "''${binaries[@]}"; do
              base=$(basename $binary)

              # Strip binary suffixes
              base=''${base%.exe}
              base=''${base%.efi}

              cdx=$(find . -name "''${base}_bin.cdx.json")

              if [ -f "$cdx" ]; then
                echo "Found SBOM for binary '$binary': $cdx"
                install -m444 "$cdx" $out/
              else
                echo "Failed to find SBOM for binary: $binary"
                exit 1
              fi
            done
          '';

          separateDebugInfo = false;
        };
      }
    );

  # use like this:
  # example = rustPlatform.buildRustPackage (finalAttrs: {
  #   pname = "example-rust";
  #   version = "0.1.0";
  #   src = lib.cleanSourceWith {
  #     filter = customFilter ../.;
  #     src = ../.;
  #   };
  #   cargoLock.lockFile = ../Cargo.lock;
  #   passthru.bombonVendoredSbom = bombon.passthruVendoredSbom.rust2 { inherit pkgs; } finalAttrs;
  # });
  rust2 =
    {
      pkgs,
      includeBuildtimeDependencies ? false,
    }:
    finalAttrs:
    pkgs.rustPlatform.buildRustPackage {
      pname = finalAttrs.pname + "-bombon-vendored-sbom";
      inherit (finalAttrs) version cargoDeps src;

      nativeBuildInputs = (finalAttrs.nativeBuildInputs or [ ]) ++ [
        pkgs.buildPackages.cargo-cyclonedx
      ];
      outputs = [ "out" ];
      phases = [
        "unpackPhase"
        "patchPhase"
        "configurePhase"
        "buildPhase"
        "installPhase"
      ];

      buildPhase =
        ''
          cargo cyclonedx \
            --spec-version 1.5 \
            --format json \
            --describe binaries \
            --target ${pkgs.stdenv.hostPlatform.rust.rustcTarget} \
        ''
        + pkgs.lib.optionalString (
          builtins.hasAttr "buildNoDefaultFeatures" finalAttrs && finalAttrs.buildNoDefaultFeatures
        ) " --no-default-features"
        + pkgs.lib.optionalString (
          builtins.hasAttr "buildFeatures" finalAttrs && builtins.length finalAttrs.buildFeatures > 0
        ) (" --features " + builtins.concatStringsSep "," finalAttrs.buildFeatures)
        + pkgs.lib.optionalString (!includeBuildtimeDependencies) " --no-build-deps";

      installPhase = ''
        mkdir -p $out
        # Collect all paths to executable files. Cargo has no good support to find this
        # and this method is very robust. The flipside is that we have to build the package
        # to generate a BOM for it.
        mapfile -d "" binaries < <(find ${finalAttrs.finalPackage} -type f -executable -print0)

        for binary in "''${binaries[@]}"; do
          base=$(basename $binary)

          # Strip binary suffixes
          base=''${base%.exe}
          base=''${base%.efi}

          cdx=$(find . -name "''${base}_bin.cdx.json")

          if [ -f "$cdx" ]; then
            echo "Found SBOM for binary '$binary': $cdx"
            install -m444 "$cdx" $out/
          else
            echo "Failed to find SBOM for binary: $binary"
            exit 1
          fi
        done
      '';

      separateDebugInfo = false;
    };

  # Add a passthru derivation to a Rust derivation `package` that generates a
  # CycloneDX SBOM.
  #
  # This could be done much more elegantly if `buildRustPackage` supported
  # finalAttrs. When https://github.com/NixOS/nixpkgs/pull/194475 lands, we can
  # most likely get rid of this.
  rust =
    package:
    {
      pkgs,
      includeBuildtimeDependencies ? false,
    }:
    package.overrideAttrs (previousAttrs: {
      passthru = (previousAttrs.passthru or { }) // {
        bombonVendoredSbom = package.overrideAttrs (previousAttrs: {
          pname = previousAttrs.pname + "-bombon-vendored-sbom";
          nativeBuildInputs = (previousAttrs.nativeBuildInputs or [ ]) ++ [
            pkgs.buildPackages.cargo-cyclonedx
          ];
          outputs = [ "out" ];
          phases = [
            "unpackPhase"
            "patchPhase"
            "configurePhase"
            "buildPhase"
            "installPhase"
          ];

          buildPhase = ''
            cargo cyclonedx \
              --spec-version 1.5 \
              --format json \
              --describe binaries \
              --target ${pkgs.stdenv.hostPlatform.rust.rustcTarget} \
          ''
          + pkgs.lib.optionalString (
            builtins.hasAttr "buildNoDefaultFeatures" previousAttrs && previousAttrs.buildNoDefaultFeatures
          ) " --no-default-features"
          + pkgs.lib.optionalString (
            builtins.hasAttr "buildFeatures" previousAttrs && builtins.length previousAttrs.buildFeatures > 0
          ) (" --features " + builtins.concatStringsSep "," previousAttrs.buildFeatures)
          + pkgs.lib.optionalString (!includeBuildtimeDependencies) " --no-build-deps";

          installPhase = ''
            mkdir -p $out

            # Collect all paths to executable files. Cargo has no good support to find this
            # and this method is very robust. The flipside is that we have to build the package
            # to generate a BOM for it.
            mapfile -d "" binaries < <(find ${package} -type f -executable -print0)

            for binary in "''${binaries[@]}"; do
              base=$(basename $binary)

              # Strip binary suffixes
              base=''${base%.exe}
              base=''${base%.efi}

              cdx=$(find . -name "''${base}_bin.cdx.json")

              if [ -f "$cdx" ]; then
                echo "Found SBOM for binary '$binary': $cdx"
                install -m444 "$cdx" $out/
              else
                echo "Failed to find SBOM for binary: $binary"
                exit 1
              fi
            done
          '';

          separateDebugInfo = false;
        });
      };
    });
}
