{ ... }:
{
  imports = [
    ./modules/tms-server.nix
  ];
  TMS_ROOT_DIR = "/tmp/tms4";
  POSTGRES_USER = "postgres";
  POSTGRES_PASSWORD = "foo";
}
