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
