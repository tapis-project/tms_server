{ ... }:
{
  perSystem =
    { inputs', config, ... }:
    {
      devShells.default = (config.craneLib.devShell.override {
        mkShell = inputs'.shell-utils.lib.shell;
      }) { };
    };
}
