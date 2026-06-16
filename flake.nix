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
    simple-flake.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
      perSystem = { pkgs, inputs', system, ... }:
        let
          craneLib = pkgs.callPackage ./nix/rust.nix {
            crane = inputs.crane;
           };
          tms-server = pkgs.callPackage ./nix/tms-server.nix {
            inherit craneLib;
          };
          shell = pkgs.callPackage ./nix/shell.nix {
            inherit craneLib;
            inherit (inputs'.shell-utils.lib) shell;
          };
        in
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ (import inputs.rust-overlay) ];
          };
          packages = {
            default = tms-server;
            my_rust = pkgs.rust-bin.stable.latest.default;
          };
          devShells = {
            default = shell;
          };
        };
    };
}
