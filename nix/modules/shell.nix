{ inputs', config, ... }:
{
  imports = [ ./rust.nix ];
  devShells.default = (config.craneLib.devShell.override {
    mkShell = inputs'.shell-utils.lib.shell;
  }) { };
}
