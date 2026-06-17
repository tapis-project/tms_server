{ lib, ... }:
{
  imports = [ ./rust.nix ];
  config = {
    perSystem = { config, pkgs, ... }:
      let
        rustc_version = builtins.readFile (pkgs.runCommand "rustc-version" { } ''
          ${config.rust-bin}/bin/rustc --version > $out
        '');
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
          config.craneLib.buildPackage (commonArgs //
            {
              inherit cargoArtifacts;
              GIT_BRANCH = config.git_branch;
              GIT_COMMIT_SHORT = config.git_commit_short;
              GIT_DIRTY = config.git_dirty;
              SOURCE_TIMESTAMP = config.source_timestamp;
              RUSTC_VERSION = config.rustc_version;
            });
      in
      {
        options = {
          git_branch = lib.mkOption {
            type = lib.types.str;
            default = "unknown";
          };
          git_commit_short = lib.mkOption {
            type = lib.types.str;
            default = "unknown";
          };
          git_dirty = lib.mkOption {
            type = lib.types.str;
            default = "unknown";
          };
          source_timestamp = lib.mkOption {
            type = lib.types.str;
            default = "unknown";
          };
          rustc_version = lib.mkOption {
            type = lib.types.str;
            default = "${rustc_version}";
          };
        };
        config = {
          packages = {
            default = tms-server;
            inherit tms-server;
          };
        };
      };
  };
}
