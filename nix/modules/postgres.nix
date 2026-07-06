{ flake-parts-lib, ... }:
{
  options.perSystem = flake-parts-lib.mkPerSystemOption
    ({ lib, ... }: {
      options = with lib.types; {
        postgres = {
          POSTGRES_USER = lib.mkOption {
            type = str;
            default = "postgres";
            description = "Admin user for Postgres";
          };
          POSTGRES_PASSWORD = lib.mkOption {
            type = str;
            default = "password";
            description = "Password for the admin user of Postgres";
          };
          POSTGRES_DB = lib.mkOption {
            type = str;
            default = "postgres";
            description = "Default database for Postgres";
          };
        };
      };
    });
  config.perSystem =
    { pkgs, config, ... }:
    {
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
              config.shell-utils.waitForPort
            ];
            text = ''
              [[ "$(id -u)" -ne "0" ]] && printf "Please, run \'${name}\` as root\n" && exit 1
              echo "Starting postgres"
              podman image trust set --type accept default
              env \
                PATH="$PATH" \
                POSTGRES_USER="${config.postgres.POSTGRES_USER}" \
                POSTGRES_PASSWORD="${config.postgres.POSTGRES_PASSWORD}" \
                POSTGRES_DB="${config.postgres.POSTGRES_DB}" \
                TMS_DB_PORT="${toString config.tms.TMS_DB_PORT}" \
                podman-compose \
                  -f ${./../../deployment/postgres/tms-postgres.yml} up \
                  --force-recreate -d
              wait-for-port "${config.tms.TMS_DB_HOST}" "${toString config.tms.TMS_DB_PORT}" 
              echo "Checking postgres connection..."
              pg_isready -h ${config.tms.TMS_DB_HOST} -p ${toString config.tms.TMS_DB_PORT} \
                -U ${config.postgres.POSTGRES_USER} -t 10
            '';
          };
          psql = pkgs.writeShellApplication {
            name = "psql";
            runtimeInputs = [ pkgs.coreutils ];
            text = ''
              env PGPASSWORD="${config.postgres.POSTGRES_PASSWORD}" \
              ${pkgs.postgresql}/bin/psql -h ${config.tms.TMS_DB_HOST} -p ${toString config.tms.TMS_DB_PORT} \
                -U ${config.postgres.POSTGRES_USER} "$@"
            '';
          };
          pgIsReady = pkgs.writeShellApplication {
            name = "pg_isready";
            text = ''
              ${pkgs.postgresql}/bin/pg_isready -h ${config.tms.TMS_DB_HOST} -p ${toString config.tms.TMS_DB_PORT} \
                -U ${config.postgres.POSTGRES_USER} "$@"
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
