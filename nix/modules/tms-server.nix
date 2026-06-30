{ self, ... }:
{
  perSystem = { lib, config, pkgs, ... }:
    let
      rustc_version = builtins.readFile (pkgs.runCommand "rustc-version" { } ''
        ${config.rust-bin}/bin/rustc --version > $out
      '');
      # Base TMS Server from the Rust code
      tms-server =
        let
          src = config.craneLib.cleanCargoSource (config.craneLib.path ./../..);
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
            meta = {
              description = "TMS Server";
              mainProgram = "tms_server";
            };
          });
      # TMS Server that bundles the `resources` directory, so it can install
      # the config files without needing the source files.
      wrapped-tms-server = pkgs.stdenv.mkDerivation {
        name = "tms-server";
        src = ./../../resources;
        nativeBuildInputs = [ pkgs.makeWrapper ];
        buildInputs = [ pkgs.rsync ];
        installPhase = ''
          mkdir -p $out/{bin,src/resources}
          rsync -a . $out/src/resources/
          makeWrapper ${lib.getExe tms-server} $out/bin/tms-server \
            --set TMS_RESOURCES_DIR $out/src/resources \
            --set TMS_ROOT_DIR ${config.TMS_ROOT_DIR} \
            --set TMS_DB_HOST ${config.TMS_DB_HOST} \
            --set TMS_DB_PORT ${toString config.TMS_DB_PORT} \
            --set TMS_DB_USER ${config.TMS_DB_USER} \
            --set TMS_DB_USER_PASSWORD ${config.TMS_DB_USER_PASSWORD} \
            --set TMS_DB_DB_NAME ${config.TMS_DB_DB_NAME}
        '';
      };
      # TMS + Postgres for local development.
      # Start Postgres with empty db, run `tms-server --install` on a fresh root
      # directory, and start `tms-server`.
      tms-server-stack-up = pkgs.writeShellApplication {
        name = "tms-server-up";
        text = ''
          ${config.packages.postgres}/bin/postgres-up
        '';
      };
      tms-server-stack-down = pkgs.writeShellApplication {
        name = "tms-server-down";
        text = ''
          ${config.packages.postgres}/bin/postgres-down
        '';
      };
      tms-server-stack = pkgs.symlinkJoin {
        name = "tms-server-stack";
        paths = [
          tms-server-stack-up
          tms-server-stack-down
        ];
      };
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
        TMS_ROOT_DIR = lib.mkOption {
          type = lib.types.str;
          default = "~/.tms";
        };
        TMS_DB_HOST = lib.mkOption {
          type = lib.types.str;
          default = "localhost";
        };
        TMS_DB_PORT = lib.mkOption {
          type = lib.types.port;
          default = 5432;
        };
        TMS_DB_DB_NAME = lib.mkOption {
          type = lib.types.str;
          default = "tmsdb";
        };
        TMS_DB_USER = lib.mkOption {
          type = lib.types.str;
          default = "tms";
        };
        TMS_DB_USER_PASSWORD = lib.mkOption {
          type = lib.types.str;
          default = "password";
        };
        TMS_SSL_CERT_PATH = lib.mkOption {
          type = lib.types.str;
        };
        TMS_SSL_KEY_PATH = lib.mkOption {
          type = lib.types.str;
        };
      };
      config = {
        packages = {
          default = wrapped-tms-server;
          inherit wrapped-tms-server tms-server tms-server-stack;
        };
        git_branch = "main";
        git_commit_short = self.shortRev or self.dirtyShortRev or "unknown";
        git_dirty = if self ? dirtyShortRev then "true" else "false";
      };
    };
}
