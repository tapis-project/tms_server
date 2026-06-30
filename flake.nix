# This flake builds the TMS Server.

{
  description = "A Nix Flake for the TMS Server";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    # simple-flake.url = "path:/Users/wmoreira/repos/simple-flake";
    simple-flake.url = "github:waltermoreira/simple-flake";
    shell-utils.url = "github:waltermoreira/shell-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ simple-flake, ... }:
    simple-flake.lib.mkFlake { inherit inputs; }
      {
        imports = [
          ./nix/modules
        ];
        config = {
          debug = true;
          systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
          perSystem = {
            imports = [
              ./nix/config.nix
            ];
          };
        };
      };
}
