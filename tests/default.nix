{ pkgs
, buildBom
}:

let
  testDerivations = with pkgs; [
    hello
    python3
  ];

  genAttrsFromDrvs = drvs: f:
    builtins.listToAttrs (map (d: pkgs.lib.nameValuePair d.pname (f d)) drvs);
in
genAttrsFromDrvs testDerivations buildBom
