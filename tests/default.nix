{ pkgs
, buildBom
}:

let
  testDerivations = with pkgs; [
    hello
    python3
    python3Packages.poetry # weird string license in buildtimeDependencies
  ];

  genAttrsFromDrvs = drvs: f:
    builtins.listToAttrs (map (d: pkgs.lib.nameValuePair d.pname (f d)) drvs);
in
genAttrsFromDrvs testDerivations buildBom
