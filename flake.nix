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
    simple-flake.lib.mkFlake { inherit inputs; }
      {
        debug = true;
        imports = [
          ./nix/rust.nix
          ./nix/tms-server.nix
          ./nix/shell.nix
        ];
        systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
        perSystem = { ... }: {
          config.git_branch = "baz";
          config.git_commit_short = self.shortRev or self.dirtyShortRev or "unknown";
          config.git_dirty = if self ? dirtyShortRev then "true" else "false";
        };
      };
}
