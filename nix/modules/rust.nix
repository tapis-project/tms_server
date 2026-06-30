{ inputs, ... }:
{
  perSystem =
    { lib, config, pkgs, system, ... }:
    {
      options = {
        rust-toolchain = lib.mkOption {
          type = lib.types.functionTo lib.types.package;
          default = pkgs: pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" ];
          };
          description = "Function from `pkgs` to a Rust toolchain (such as oxalica's overlay)";
        };
        rust-bin = lib.mkOption {
          type = lib.types.package;
          default = config.rust-toolchain pkgs;
          readOnly = true;
          description = "A Rust toolchain (such as oxalica's overlay)";
        };
        craneLib = lib.mkOption {
          type = lib.types.anything;
          default = (inputs.crane.mkLib pkgs).overrideToolchain config.rust-toolchain;
        };
      };
      config = {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [ (import inputs.rust-overlay) ];
        };
      };
    };
}
