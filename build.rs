#![forbid(unsafe_code)]

fn main() {
    build_data::set_GIT_BRANCH();
    build_data::set_GIT_COMMIT_SHORT();
    build_data::set_GIT_DIRTY();
    build_data::set_SOURCE_TIMESTAMP();  // Using BUILD_TIMESTAMP makes build unreproducible.
    build_data::set_RUSTC_VERSION();
    
    // Tells cargo not to rebuild build.rs during debug builds when other files change.
    // This speeds up development builds.
    //build_data::no_debug_rebuilds();
}