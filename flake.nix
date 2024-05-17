# This flake builds the TMS Server.

{
  description = "A Nix Flake for the TMS Server";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , crane
    , rust-overlay
    }:

    flake-utils.lib.eachDefaultSystem (system:
    let
      # Standard nix packages
      # pkgs = nixpkgs.legacyPackages.${system};
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };

      # Crane is used for building the Rust Camers Traps Engine        
      craneLib = crane.lib.${system};

      # Build the TMS server
      tms_server =
        craneLib.buildPackage
          {
            src = craneLib.cleanCargoSource ./.;
            nativeBuildInputs = with pkgs; [
              pkg-config
              pkgs.sqlx-cli
              git
            ];
            GIT_BRANCH = "foo";
            GIT_COMMIT_SHORT = "foo";
            GIT_DIRTY = "foo";
            SOURCE_TIMESTAMP = "foo";
            RUSTC_VERSION = "foo";
          };
    in
    rec {
      packages = {
        tms_server_package = tms_server;
        default = packages.tms_server_package;
      };

    });

}
