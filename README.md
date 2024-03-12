# Bombon

Automagically build CycloneDX Software Bills of Materials (SBOMs) for Nix packages!

Bombon generates CycloneDX v1.4 SBOMs which aim to be compliant with:

- The German [Technical Guideline TR-03183][] of the Federal Office for Information
  Security (BSI)
- The US [Executive Order 14028][]

If you find that they aren't compliant in any way, please open an issue!

[Technical Guideline TR-03183]: https://www.bsi.bund.de/SharedDocs/Downloads/EN/BSI/Publications/TechGuidelines/TR03183/BSI-TR-03183-2.pdf?__blob=publicationFile&v=5
[Executive Order 14028]: https://www.nist.gov/itl/executive-order-14028-improving-nations-cybersecurity/software-security-supply-chains-software-1

## Getting Started

### Flakes

```sh
nix flake init -t github:nikstur/bombon
```

Or manually copy this to `flake.nix` in your repository:

```nix
# file: flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    bombon.url = "github:nikstur/bombon";
    bombon.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, bombon }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      packages.${system}.default = bombon.lib.${system}.buildBom pkgs.hello { };
    };
}
```

### Niv

```sh
niv init
niv add nikstur/bombon
```

```nix
# file: default.nix
let
  sources = import ./nix/sources.nix { };
  pkgs = import sources.nixpkgs { };
  bombon = import sources.bombon { inherit pkgs; };
in
bombon.buildBom pkgs.hello { }
```

## Options

`buildBom` accepts options as an attribute set. All attributes are optional:

- `extraPaths`: a list of store paths to also consider for the SBOM. This is
  useful when you build images that discard their references (e.g. with
  [`unsafeDiscardReferences`](https://nixos.org/manual/nix/stable/language/advanced-attributes#adv-attr-unsafeDiscardReferences)
  but you still want their contents to appear in the SBOM. The `extraPaths`
  will appear as components of the main derivation.
- `includeBuildtimeDependencies`: boolean flag to include buildtime dependencies in output.

Example:

```nix
bombon.lib.${system}.buildBom pkgs.hello {
  extraPaths = [ pkgs.git ];
  includeBuildtimeDependencies = true;
}
```

## Contributing

During development, the Nix Repl is a convenient and quick way to test changes.
Start the repl, loading your local version of nixpkgs.

```sh
nix repl <nixpkgs>
```

Inside the repl, load the bombon flake and build the BOM for a package you
are interested in.

```nix-repl
:l .
:b lib.x86_64-linux.buildBom python3 { }
```

Remember to re load the bombon flake every time you made changes to any of the
source code.

## Acknowledgements

The way dependencies are retrieved using Nix is heavily influenced by this
[blog article from Nicolas
Mattia](https://www.nmattia.com/posts/2019-10-08-runtime-dependencies.html).
