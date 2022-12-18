# Bombon

Automagically build CycloneDX Software Bills of Materials (SBOMs) for Nix packages!

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
  bombon = import sources.bombon;
  system = "x86_64-linux";
in
bombon.lib.${system}.buildBom pkgs.hello { }
```

## Options

`buildBom` accepts options as an attribute set. All attributes are optional:

- `includeBuildtimeDependencies`: include buildtime dependencies in output.

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
