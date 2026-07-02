{ self, ... }:
{
  perSystem = { lib, config, pkgs, ... }:
    let
      rustc_version = builtins.readFile (pkgs.runCommand "rustc-version" { } ''
        ${config.rust-bin}/bin/rustc --version > $out
      '');
      initDb = pkgs.stdenv.mkDerivation {
        name = "initDb";
        src = ./../../deployment/postgres;
        buildPhase = ''
          cp tms_init_db.sh $out
          patchShebangs $out
        '';
      };
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
            GIT_BRANCH = config.tms.git_branch;
            GIT_COMMIT_SHORT = config.tms.git_commit_short;
            GIT_DIRTY = config.tms.git_dirty;
            SOURCE_TIMESTAMP = config.tms.source_timestamp;
            RUSTC_VERSION = config.tms.rustc_version;
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
            --set TMS_ROOT_DIR ${config.tms.TMS_ROOT_DIR} \
            --set TMS_DB_HOST ${config.tms.TMS_DB_HOST} \
            --set TMS_DB_PORT ${toString config.tms.TMS_DB_PORT} \
            --set TMS_DB_USER ${config.tms.TMS_DB_USER} \
            --set TMS_DB_USER_PASSWORD ${config.tms.TMS_DB_USER_PASSWORD} \
            --set TMS_DB_DB_NAME ${config.tms.TMS_DB_DB_NAME}
        '';
      };
      # TMS + Postgres for local development.
      # Start Postgres with empty db, run `tms-server --install` on a fresh root
      # directory, and start `tms-server`.
      tms-server-stack-up = pkgs.writeShellApplication {
        name = "tms-server-up";
        runtimeInputs = with pkgs; [
          mktemp
        ];
        text = ''
          command -v sudo >/dev/null 2>&1 || (printf "Need \`sudo\` to run postgres"; exit 1)
          tms-server-down () {
            sudo ${config.packages.postgres}/bin/postgres-down
          }
          trap tms-server-down EXIT
          sudo ${config.packages.postgres}/bin/postgres-up
          echo "Initializing database"
          env \
            PATH="$PATH" \
            TMS_DB_HOST="${config.tms.TMS_DB_HOST}" \
            TMS_DB_PORT="${toString config.tms.TMS_DB_PORT}" \
            POSTGRES_USER="${config.postgres.POSTGRES_USER}" \
            POSTGRES_PASSWORD="${config.postgres.POSTGRES_PASSWORD}" \
            TMS_DB_USER="${config.tms.TMS_DB_USER}" \
            TMS_DB_USER_PASSWORD="${config.tms.TMS_DB_USER_PASSWORD}" \
            TMS_DB_DB_NAME="${config.tms.TMS_DB_DB_NAME}" \
            ${initDb}
          TEMP=$(mktemp -d)
          ${wrapped-tms-server}/bin/tms-server --install --root-dir "$TEMP"
          ${pkgs.gum}/bin/gum style \
            --foreground 212 --border-foreground 212 --border double \
            --align center --width 50 --margin "1 2" --padding "2 4" \
            "TMS Server is running in root-dir = $TEMP"
          ${wrapped-tms-server}/bin/tms-server --root-dir "$TEMP"
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
        tms = {
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
      };
      config = {
        packages = {
          #default = wrapped-tms-server;
          inherit wrapped-tms-server tms-server tms-server-stack;
        };
        tms.git_branch = "main";
        tms.git_commit_short = self.shortRev or self.dirtyShortRev or "unknown";
        tms.git_dirty = if self ? dirtyShortRev then "true" else "false";
      };
    };
}
