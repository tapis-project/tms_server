{ ... }:
{
  perSystem =
    { lib, pkgs, config, ... }:
    {
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
          postgresDown = pkgs.writeShellApplication rec {
            name = "postgres-down";
            runtimeInputs = with pkgs; [
              coreutils
              podman-compose
              podman
            ];
            text = ''
              [[ "$(id -u)" -ne "0" ]] && printf "Please, run \`${name}\` as root\n" && exit 1
              ${pkgs.podman-compose}/bin/podman-compose \
                  -f ${./../../deployment/postgres/tms-postgres.yml} down -v
            '';
          };
          postgresUp = pkgs.writeShellApplication rec {
            name = "postgres-up";
            runtimeInputs = with pkgs; [
              podman-compose
              podman
              postgresql
              coreutils
            ];
            text = ''
              [[ "$(id -u)" -ne "0" ]] && printf "Please, run \'${name}\` as root\n" && exit 1
              echo "Starting postgres"
              ${pkgs.podman}/bin/podman image trust set --type accept default
              env \
                PATH="$PATH" \
                POSTGRES_USER="${config.POSTGRES_USER}" \
                POSTGRES_PASSWORD="${config.POSTGRES_PASSWORD}" \
                POSTGRES_DB="${config.POSTGRES_DB}" \
                TMS_DB_PORT="${toString config.TMS_DB_PORT}" \
                ${pkgs.podman-compose}/bin/podman-compose \
                  -f ${./../../deployment/postgres/tms-postgres.yml} up \
                  --force-recreate -d
              sleep 1
              echo "Waiting max 10 seconds for Postgres to start..."
              ${pkgs.postgresql}/bin/pg_isready -h ${config.TMS_DB_HOST} -p ${toString config.TMS_DB_PORT} \
                -U ${config.POSTGRES_USER} -t 10
            '';
          };
          psql = pkgs.writeShellApplication {
            name = "psql";
            text = ''
              ${pkgs.postgresql}/bin/psql -h ${config.TMS_DB_HOST} -p ${toString config.TMS_DB_PORT} \
                -U ${config.POSTGRES_USER} "$@"
            '';
          };
          postgres = pkgs.symlinkJoin {
            name = "postgres";
            paths = [
              postgresUp
              postgresDown
              psql
            ];
          };
        in
        {
          packages = {
            inherit postgres;
          };
        };
    };
}
