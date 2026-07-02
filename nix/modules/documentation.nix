{ inputs, ... }:
{
  imports = [
    inputs.flake-parts-website.flakeModules.empty-site
  ];
  config = {
    flake.flakeModule = import ./default.nix;
    perSystem = { ... }: {
      render.inputs.tms = {
        flake = inputs.self;
        baseUrl = "https://github.com/tapis-project/tms_server/blob/main";
        intro = ''
          My introduction.
        '';
        installation = ''
          My Installation instructions.
        '';
      };
    };
  };
}
