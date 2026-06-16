{ lib, ... }:
{
  imports = [ ./rust.nix ];
  config = {
    perSystem = { config, pkgs, ... }:
      let
        tms-server =
          let
            src = config.craneLib.cleanCargoSource (config.craneLib.path ./..);
            commonArgs = {
              inherit src;
              buildInputs = with pkgs; [
                pkg-config
                sqlx-cli
                openssl
                git
              ] ++ lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
            };
            cargoArtifacts = config.craneLib.buildDepsOnly commonArgs;
          in
          builtins.trace config
            (config.craneLib.buildPackage (commonArgs //
              {
                inherit cargoArtifacts;
                GIT_BRANCH = "foo";
                GIT_COMMIT_SHORT = "foo";
                GIT_DIRTY = "foo";
                SOURCE_TIMESTAMP = "foo";
                RUSTC_VERSION = "foo";
              }));
      in
      {
        packages = {
          default = tms-server;
          inherit tms-server;
        };
      };
  };
}
