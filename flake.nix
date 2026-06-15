# This flake builds the TMS Server.

{
  description = "A Nix Flake for the TMS Server";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    simple-flake.url = "github:waltermoreira/simple-flake";
    shell-utils.url = "github:waltermoreira/shell-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = inputs@{ self, simple-flake, ... }:
    simple-flake.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
      perSystem = { pkgs, inputs', system, ... }:
        let
          craneLib = inputs.crane.mkLib pkgs; 
          tms-server = pkgs.callPackage ./nix/tms-server.nix { 
            inherit craneLib;
          };
          shell = pkgs.callPackage ./nix/shell.nix { 
            inherit craneLib;
            inherit (inputs'.shell-utils.lib) shell;
          };
        in
        {
          packages = { 
            default = tms-server;
          };
          devShells = { 
            default = shell;
          };
        };
    };
}
