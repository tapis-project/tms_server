{ lib, stdenv, pkgs, craneLib, pkg-config, sqlx-cli, openssl, git, ... }:
craneLib.buildPackage {
  src = craneLib.cleanCargoSource ./..;
  nativeBuildInputs = [
    pkg-config
    sqlx-cli
    openssl
    git
  ];
  buildInputs = [ ] ++ lib.optionals stdenv.isDarwin [ pkgs.libiconv ];
}
