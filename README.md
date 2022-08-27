# Bombon

Easily generate *comprehensive* Bill of Materials of your Software leveraging
the power of Nix!

## Getting Started

### Flakes

```nix
{
  inputs = {
    bombon.url = "github:nikstur/bombon";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, nixpkgs, bombon }:
    let
      system = "x86_64-linux";
      hello = nixpkgs.legacyPackages.${system}.hello;
    in
    {
      packages.${system} = {
        hello-bom = bombon.lib.${system}.generateBom hello; 
      };
    };
}
```
