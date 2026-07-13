{ inputs, flake-parts-lib, ... }:
{
  imports = [
    inputs.flake-parts-website.flakeModules.empty-site
  ];
  options.perSystem = flake-parts-lib.mkPerSystemOption
    ({ lib, pkgs, config, ... }:
      let
        unusedPort = pkgs.writeShellApplication {
          name = "find-unused-port";
          runtimeInputs = [ pkgs.python3 ];
          text = ''
            python3 -c 'import socket; s=socket.socket(); s.bind(("", 0)); print(s.getsockname()[1]); s.close()'
          '';
        };
        mdBookProject = pkgs.stdenv.mkDerivation {
          name = "mdBook-project";
          buildInputs = [ pkgs.mdbook ];
          src = ./.;
          buildPhase = ''
            mkdir -p $out
            cd $out
            mdbook init --force --title "${config.render.inputs.tms.title}"
            cat <<EOF >src/SUMMARY.md
              # Summary

              - [Options](./options.md)
            EOF
            cp ${config.packages.generated-docs-tms}/options.md src
            rm src/chapter_1.md
            mdbook build
          '';
        };
        serve = pkgs.writeShellApplication {
          name = "docs-serve";
          runtimeInputs = [ pkgs.python3 pkgs.coreutils pkgs.xdg-utils ];
          text = ''
            cd ${mdBookProject}/book
            PORT="$(${unusedPort}/bin/find-unused-port)"
            trap 'kill $(jobs -p) && echo "Documentation server stopped"' EXIT
            python3 -m http.server "$PORT" &
            ${pkgs.gum}/bin/gum style \
            --foreground 212 --border-foreground 212 --border double \
            --align center --width 50 --margin "1 2" --padding "2 4" \
            "Documentation available at http://localhost:$PORT"
            xdg-open http://localhost:"$PORT"
            sleep infinity
          '';
        };
      in
      {
        options = {
          documentation = {
            port = lib.mkOption {
              type = lib.nullOr lib.types.port;
              default = null;
              defaultText = "Find a free open port in the host machine";
              description = "Port in localhost where to serve the documentation";
            };
            unusedPort = lib.mkOption {
              type = lib.types.package;
              default = unusedPort;
            };
            mdBookProject = lib.mkOption {
              type = lib.types.package;
              default = mdBookProject;
            };
            serve = lib.mkOption {
              type = lib.types.package;
              default = serve;
            };
          };
        };
      });
  config = {
    flake.flakeModule = import ./default.nix;
    perSystem = { config, pkgs, ... }: {
      packages = {
        inherit (config.documentation) unusedPort mdBookProject;
        docs-serve = config.documentation.serve;
      };
      render.inputs.tms = {
        flake = inputs.self;
        baseUrl = "https://github.com/tapis-project/tms_server/blob/${config.tms.git_branch}";
        intro = ''
          My introduction.
        '';
        installation = ''
          My Installation instructions.
        '';
        title = "TMS Server";
      };
    };
  };
}
