#![forbid(unsafe_code)]

fn main() {
    // Note that the various git-related build_data functions fail whenever git is not on the path and/or
    // the .git folder is not available. In particular, all of the above calls to set_GIT_* will panic
    // in a Nix build environment, so we must catch/unwind the panic and replace with an environment variable
    // read in at run time (it won't be available at compile time when not building in Nix).

    // When building in Nix, the variables are computed in the flake and set directly in the environment.
    #[allow(clippy::let_unit_value, clippy::redundant_closure)]
    let _ = std::panic::catch_unwind(|| build_data::set_GIT_BRANCH()).unwrap_or_else(|_| {
        std::env::var("GIT_BRANCH").unwrap();
    });

    #[allow(clippy::let_unit_value, clippy::redundant_closure)]
    let _ = std::panic::catch_unwind(|| build_data::set_GIT_COMMIT_SHORT()).unwrap_or_else(|_| {
        std::env::var("GIT_COMMIT_SHORT").unwrap();
    });

    #[allow(clippy::let_unit_value, clippy::redundant_closure)]
    let _ = std::panic::catch_unwind(|| build_data::set_GIT_DIRTY()).unwrap_or_else(|_| {
        std::env::var("GIT_DIRTY").unwrap();
    });

    #[allow(clippy::let_unit_value, clippy::redundant_closure)]
    let _ = std::panic::catch_unwind(|| build_data::set_SOURCE_TIMESTAMP()).unwrap_or_else(|_| {
        std::env::var("SOURCE_TIMESTAMP").unwrap();
    });

    #[allow(clippy::let_unit_value, clippy::redundant_closure)]
    let _ = std::panic::catch_unwind(|| build_data::set_RUSTC_VERSION()).unwrap_or_else(|_| {
        std::env::var("RUSTC_VERSION").unwrap();
    });

    // Tells cargo not to rebuild build.rs during debug builds when other files change.
    // This speeds up development builds.
    build_data::no_debug_rebuilds();
}
