# Running TMS Server with Nix

## Goals

The goal of the Nix code is to provide a streamlined experience for a developer to start
a local instance of the TMS Server. It should be a "single button" experience to spawn
the full stack (server + database) with no installation of dependencies. And it
should also provide a unified configuration file for the stack.

The instance of the stack should be described in a single source of truth (the `flake`
file). In development, it should keep no state outside of it. The database gets
recreated on each run.

In the near future, the plan is to propose and similar framework for deployment with
the following goals:
- Single source of truth for build and deployment
- Single persistent state in the database. This means that no configuration happens
  outside of the flake or the database.

## Usage

### Requirements

The requirements for running the TMS Server stack are:

1. A Linux machine (x86_64 and aarch64 are both supported).
2. `sudo` enabled for the user (this requirement will be removed in the next iteration).
3. A [nix](https://nixos.org/download/#nix-install-linux) installation.

### Running the server

If the promise of "single button" experience holds, the previous requirements should be
enough to execute (note that no cloning or source of TMS Server is needed):
```bash
nix run "github:tapis-project/tms_server?ref=wm/nix"
```
If all goes well, the database and the server should be running. The `ref=wm/nix`
indicates a branch in the repository. When we merge this code into the main branch,
the command will simply be `nix run "github:tapis-project/tms-server"`.

Of course, for developing we need to have the Git repository. In a local clone,
switch to the branch `wm/nix` and run
```bash
nix run
```
This should run the stack, picking up any local changes in the working directory.

#### Configuring

The master configuration file is `nix/config.nix`. Check the documentation by following
the [rendering instructions](#rendering-the-documentation).

### Spawning a development shell

The Nix code provides a development shell with all the utilities and dependencies for
developing and building the TMS Server. 

Enter the environment with:
```
nix develop
```
This environment provides the Rust toolchain, the database, and the environment
variables specified in the config file. For example, the command `psql`, with no
paramaters, connects automatically to the right database server, with the right
user, and the right password.

### Rendering the documentation

The flake provides automatic generation of documentation for the configuration options.
Running
```bash
nix run .#docs-serve
```
will build and open a browser window the documentation of all the options available
in `config.nix`.

If the developer is inside the [development environment](#spawning-a-development-shell),
then they can just run `docs-serve`.

## Extending the Nix modules

The single point of entry for nix is `flake.nix`. The flake loads a set of modules,
each of those providing differerent capabilities. A module declares a set of options
(render them with `docs-serve`, or inspect the code in `nix/modules/`)
and provides some functionality.

For example, the module `nix/modules/tms-server.nix` provides the functionality for 
building the TMS Server and the different variations of it hooked to the database.

## To do

- Expand documentation.
- Convert the development postgres db from container to pure Nix to avoid `sudo`.
- Design and implement a deployment strategy
- Introduce the same framework for the TMS Portal, TMS Providers, and hook them
  with a master flake.