{ crane, pkgs, ... }:
(crane.mkLib pkgs).overrideToolchain (
  p: p.rust-bin.stable.latest.default.override {
    extensions = [ "rust-src" ];
  }
)
