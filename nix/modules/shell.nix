{ ... }:
{
  perSystem =
    { inputs', config, ... }:
    {
      devShells.default = (config.rust.craneLib.devShell.override {
        mkShell = inputs'.shell-utils.lib.shell;
      }) { 
        name = "TMS-Server-Dev";
        extraInitRc = ''
          alias sudo='\sudo env PATH="$PATH" HOME="$HOME"'
        '';
        packages = with config.packages; [
          tms-server-stack
          postgres
          docs-serve
        ];
      };
    };
}
