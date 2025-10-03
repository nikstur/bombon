let
  sources = import ../lon.nix;
  pre-commit = import sources."pre-commit";
  pkgs = import sources.nixpkgs { };

  globalExcludes = [
    "lon.nix"
    "sources.nix"
  ];
in
pre-commit.run {
  src = pkgs.nix-gitignore.gitignoreSource [ ] ../.;
  hooks = {
    nixfmt-rfc-style = {
      enable = true;
      excludes = globalExcludes;
    };
    statix = {
      enable = true;
      settings.ignore = globalExcludes;
      excludes = globalExcludes;
    };
    deadnix = {
      enable = true;
      excludes = globalExcludes;
    };
    typos.enable = true;
  };
}
