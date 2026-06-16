# This flake builds the TMS Server.

{
  description = "A Nix Flake for the TMS Server";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    simple-flake.url = "github:waltermoreira/simple-flake";
    shell-utils.url = "github:waltermoreira/shell-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ self, simple-flake, ... }:
    simple-flake.lib.mkFlake { inherit inputs; } ({ flake-parts-lib, ... }:
      let
        inherit (flake-parts-lib) importApply;
        initModule = importApply ./nix/init.nix { inherit inputs; };
      in
      {
        debug = true;
        imports = [
          initModule
          ./nix/rust.nix
          ./nix/tms-server.nix
          ./nix/shell.nix
        ];
        systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
        # perSystem = { config, pkgs, inputs', system, ... }:
        #   let
        #     shell = pkgs.callPackage ./nix/shell.nix {
        #       # inherit craneLib;
        #       inherit (inputs'.shell-utils.lib) shell;
        #     };
        #   in
        #   {
        #     packages.default = config.tms-server;
        #     devShells = {
        #       default = shell;
        #     };
        #   };
      });
}
