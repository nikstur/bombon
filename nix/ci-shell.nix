let
  sources = import ../lon.nix;
  pkgs = import sources.nixpkgs { };
in
pkgs.mkShell {
  packages = [
    pkgs.nix-eval-jobs
    pkgs.jq
  ];

  shellHook = ''
    eval-checks() {
      nix-eval-jobs release.nix --check-cache-status | jq -s 'map({attr, isCached})'
    }
  '';
}
