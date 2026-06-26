{ lib, ... }:
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
  config = {
    perSystem = { config, pkgs, ... }:
      let
        postgres = pkgs.stdenv.mkDerivation {

        };
      in
      {
        packages = {
          inherit postgres;
        };
      };
  };
}
