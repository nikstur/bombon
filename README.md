# Bombon

Easily generate Software Bill of Materials using Nix!

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

Remember to re load the bombon flake everytime you made changes to any of the
source code.
