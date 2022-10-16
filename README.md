# Bombon

Easily generate Software Bill of Materials from Nix packages!

## Getting Started

### Flakes

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    bombon.url = "github:nikstur/bombon";
  };

  outputs = { self, nixpkgs, bombon }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      packages.${system}.helloBom = bombon.lib.${system}.generateBom pkgs.hello; 
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
{
  helloBom = bombon.lib.${system}.generateBom pkgs.hello; 
}
```

## Contributing

During development, the Nix Repl is a convenient and quick way to test changes.
Start the repl, loading your local version of nixpkgs.

```sh
nix repl <nixpkgs>
```

Inside the repl, load the bombon flake and generate the BOM for a package you
are interested in.

```nix-repl
:l .
:b lib.x86_64-linux.generateBom python3
```

Remember to re load the bombon flake every time you made changes to any of the
source code.

## Acknowledgements

The way dependencies are retrieved using Nix is heavily influenced by this
[blog article from Nicolas
Mattia](https://www.nmattia.com/posts/2019-10-08-runtime-dependencies.html).
