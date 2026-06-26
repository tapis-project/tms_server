{ ... }:
{
  imports = [ ./rust.nix ];
  config = {
    perSystem = { config, inputs', ... }: {
      devShells.default = (config.craneLib.devShell.override {
        mkShell = inputs'.shell-utils.lib.shell;
      }) { };
    };
  };
}
