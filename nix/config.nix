# Configuration module for the TMS Server stack

{ ... }:
{
  # TMS Server specific options
  # (All the options have types and defaults. See ./modules/tms-server.nix)
  #
  TMS_ROOT_DIR = "/tmp/tms4";
  # TMS_DB_HOST = "localhost";
  # TMS_DB_PORT = 4323;
  # TMS_DB_DB_NAME = "tmsdb";
  # TMS_DB_USER = "tms";
  # TMS_DB_USER_PASSWORD = "password";
  # TMS_SSL_CERT_PATH = "";
  # TMS_SSL_KEY_PATH = "";

  # Postgres speficic options (for local development)
  # (See types and defaults in ./modules/postgres.nix)
  #
  POSTGRES_USER = "postgres";
  POSTGRES_PASSWORD = "foo";
  POSTGRES_DB = "postgres";
}
