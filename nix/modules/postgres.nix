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
              netcat
            ];
            text = ''
              [[ "$(id -u)" -ne "0" ]] && printf "Please, run \'${name}\` as root\n" && exit 1
              echo "Starting postgres"
              podman image trust set --type accept default
              env \
                PATH="$PATH" \
                POSTGRES_USER="${config.POSTGRES_USER}" \
                POSTGRES_PASSWORD="${config.POSTGRES_PASSWORD}" \
                POSTGRES_DB="${config.POSTGRES_DB}" \
                TMS_DB_PORT="${toString config.TMS_DB_PORT}" \
                podman-compose \
                  -f ${./../../deployment/postgres/tms-postgres.yml} up \
                  --force-recreate -d
              retries=0
              while ! nc -z "${config.TMS_DB_HOST}" "${toString config.TMS_DB_PORT}"; do 
                if [ $retries -gt 5 ]; then
                  echo "postgres did not start"
                  exit 1
                fi
                echo "no connection, trying again..."
                sleep 1 
                retries=$((retries+1))
              done
              echo "Checking postgres connection..."
              pg_isready -h ${config.TMS_DB_HOST} -p ${toString config.TMS_DB_PORT} \
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
          pgIsReady = pkgs.writeShellApplication {
            name = "pg_isready";
            text = ''
              ${pkgs.postgresql}/bin/pg_isready -h ${config.TMS_DB_HOST} -p ${toString config.TMS_DB_PORT} \
                -U ${config.POSTGRES_USER} "$@"
            '';
          };
          postgres = pkgs.symlinkJoin {
            name = "postgres";
            paths = [
              postgresUp
              postgresDown
              psql
              pgIsReady
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
