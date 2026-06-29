{ lib, pkgs, config, ... }:
{
  imports = [
    ./tms-server.nix
  ];
  options = with lib.types; {
    POSTGRES_USER = lib.mkOption {
      type = str;
      default = "postgres";
    };
    POSTGRES_PASSWORD = lib.mkOption {
      type = str;
      default = "password";
    };
    POSTGRES_DB = lib.mkOption {
      type = str;
      default = "postgres";
    };
  };
  config =
    let
      initDb = pkgs.stdenv.mkDerivation {
        name = "initDb";
        src = ./../../deployment/postgres;
        buildPhase = ''
          cp tms_init_db.sh $out
          patchShebangs $out
        '';
      };
      postgresDown = pkgs.writeShellApplication {
        name = "postgres-down";
        text = ''
          ${pkgs.podman-compose}/bin/podman-compose \
              -f ${./../../deployment/postgres/tms-postgres.yml} down -v
        '';
      };
      postgresUp = pkgs.writeShellApplication {
        name = "postgres-up";
        runtimeInputs = with pkgs; [
          podman-compose
          podman
          postgresql
          coreutils
        ];
        text = ''
          echo "Starting postgres"
          ${pkgs.podman}/bin/podman image trust set --type accept default
          env \
            PATH="$PATH" \
            POSTGRES_USER="${config.POSTGRES_USER}" \
            POSTGRES_PASSWORD="${config.POSTGRES_PASSWORD}" \
            POSTGRES_DB="${config.POSTGRES_DB}" \
            ${pkgs.podman-compose}/bin/podman-compose \
              -f ${./../../deployment/postgres/tms-postgres.yml} up \
                --force-recreate -d
          sleep 1
          echo "----- Waiting max 10 seconds for Postgres to start"
          ${pkgs.postgresql}/bin/pg_isready -h localhost -p 5432 -U ${config.POSTGRES_USER} -t 10
          echo "----- After waiting"
          echo "Initializing database"
          env \
            PATH="$PATH" \
            TMS_DB_HOST="${config.TMS_DB_HOST}" \
            TMS_DB_PORT="${toString config.TMS_DB_PORT}" \
            POSTGRES_USER="${config.POSTGRES_USER}" \
            POSTGRES_PASSWORD="${config.POSTGRES_PASSWORD}" \
            TMS_DB_USER="${config.TMS_DB_USER}" \
            TMS_DB_USER_PASSWORD="${config.TMS_DB_USER_PASSWORD}" \
            TMS_DB_DB_NAME="${config.TMS_DB_DB_NAME}" \
            ${initDb}
        '';
      };
      postgres = pkgs.symlinkJoin {
        name = "postgres";
        paths = [
          postgresUp
          postgresDown
        ];
      };
    in
    {
      packages = {
        inherit postgres;
      };
    };
}
