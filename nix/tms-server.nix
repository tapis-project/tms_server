{ lib, stdenv, pkgs, craneLib, pkg-config, sqlx-cli, openssl, git, ... }:
let
  src = craneLib.cleanCargoSource (craneLib.path ./..);
  commonArgs = {
    inherit src;
    buildInputs = [
      pkg-config
      sqlx-cli
      openssl
      git
    ] ++ lib.optionals stdenv.isDarwin [ pkgs.libiconv ];
  };
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
craneLib.buildPackage (commonArgs //
{
  inherit cargoArtifacts;
  GIT_BRANCH = "foo";
  GIT_COMMIT_SHORT = "foo";
  GIT_DIRTY = "foo";
  SOURCE_TIMESTAMP = "foo";
  RUSTC_VERSION = "foo";
})
