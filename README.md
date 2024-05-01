# tms_server

Trust Manager System (TMS) web server

TMS is currently in prototype form and is being developed for a Minimal Viable Product release.  As development proceeds the SQLite database schema may change.  To upgrade to the new schema delete all tms.db* files from the top-level directory (all content will be lost).  TMS will automatically create a new database with the updated schema when run.

To run the server from the command line, type "cargo run".

To run the server from within Visual Studio, press the *Run* hotspot above the main() function in main.rs.

The automatically generate livedocs can be accessed by pointing your browser to https://localhost:3000.  The various APIs can be executed by filling in form data.
