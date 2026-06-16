{ topInputs, lib, ... }:
{
  config = {
    perSystem = { pkgs, system, ... }: {
      options = {
        craneLib = lib.mkOption {
          type = lib.types.anything;
          default = (topInputs.crane.mkLib pkgs).overrideToolchain (
            p: p.rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" ];
            }
          );
        };
      };
      config = {
        _module.args.pkgs = import topInputs.nixpkgs {
          inherit system;
          overlays = [ (import topInputs.rust-overlay) ];
        };
      };
    };
  };
}
