{ inputs, flake-parts-lib, ... }:
{
  options = {
    perSystem = flake-parts-lib.mkPerSystemOption
      ({ config, lib, pkgs, ... }: {
        options = {
          rust-toolchain = lib.mkOption {
            type = lib.types.functionTo lib.types.package;
            default = pkgs: pkgs.rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" ];
            };
            defaultText = lib.literalMD "default for rust toolchain";
            description = "Function from `pkgs` to a Rust toolchain (such as oxalica's overlay)";
          };
          rust-bin = lib.mkOption {
            type = lib.types.package;
            default = config.rust-toolchain pkgs;
            defaultText = lib.literalMD "default for rust-bin";
            readOnly = true;
            description = "A Rust toolchain (such as oxalica's overlay)";
          };
          craneLib = lib.mkOption
            {
              type = lib.types.anything;
              default = (inputs.crane.mkLib pkgs).overrideToolchain config.rust-toolchain;
              defaultText = lib.literalMD "the value of the `contents` option";
              description = "the crane lib";
            };
        };
      });
  };
  config = {
    perSystem =
      { system, ... }:
      {
        config = {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ (import inputs.rust-overlay) ];
          };
        };
      };
  };
}
