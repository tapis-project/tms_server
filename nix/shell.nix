{ craneLib, shell, ... }:
let
  cargoShell = craneLib.devShell.override {
    mkShell = shell;
  };
in
cargoShell { }
