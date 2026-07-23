# Locally Installing and Testing TMS Server version 0.4

## Overview
This document describes how to set up and natively install TMS Server (TMSS) for local testing.

## One-Time Installation Prerequisite Procedures
Perform the following one-time installation steps prior to installing TMSS for the first time.

### Install PostgreSQL
This may be installed and running almost anywhere. The simplest option is to install locally as a docker deployment.
Please see files under the directory `deployment/postgres` for an example docker compose file and scripts that may
be used to deploy a local postgres server. In order to use the scripts in this repo you will need to have the
postgres admin user as `postgres` and save the admin password for later use when initializing the DB for TMSS.

### Install rust and clone the repository
For example:
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
mkdir $HOME/src
cd $HOME/src
git clone https://github.com/tapis-project/tms_server.git
```

### Initialize the PostgreSQL database
Prior to installing TMSS Server for the first time the DB must be initialized by creating the database and schema.
The database will be named `tmsdb` and the schema (aka user) will be named `tms`. Note that if this is a re-installation
and you wish to destructively remove a previous install you may use the script located at
`deployment/postgres/tms_drop.sh`. To initialize the DB you will need to choose a DB user password, set two environment
variables and run the init script, as follows:
```
cd $HOME/src/tms_server
export POSTGRES_PASSWORD=<pg_password>
export TMS_DB_USER_PASSWORD=<tms_user_password>
./deployment/postgres/tms_init_db.sh
```

If the PostgreSQL deployment is not running on `localhost` at port `5432`, then the environment variables `TMS_DB_HOST`
and `TMS_DB_PORT` may be used to override the settings.

## Environment variables used during installation

### Required
The following environment variables are required when installing TMSS:
- POSTGRES_PASSWORD
  - Password for the postgres admin user `postgres`
- TMS_DB_USER_PASSWORD
  - Password for the TMS DB user
- TMS_SSL_CERT_PATH
  - Path to the SSL full-chain certificate file in PEM format that is loaded at startup.
  - Example: $HOME/src/tms_server/resources/certs_self/cert.pem
- TMS_SSL_KEY_PATH
  - Path to the private key file in PEM format associated with the server SSL certificate.
  - Example: $HOME/src/tms_server/resources/certs_self/key.pem

### Optional
Other env variables that can be set to override defaults:
- TMS_DB_HOST
  - Host server running PostgreSQL. default = localhost
- TMS_DB_PORT
  - Port at which PostgreSQL server is running. default = 5432 :

Other less common env variable overrides:
- TMS_ROOT_DIR
  - Location of `certs/`, `config/`, `logs/`, `migrations/`. default = $HOME/.tms
- TMS_LOCAL_DIR
  - Location of install output and optional custom `tms.toml`, `log4rs.yml`. default = $HOME/tms_local

NOTE: It is strongly recommended that *TMS_LOCAL_DIR* be left as the default or set to a directory
outside of *TMS_ROOT_DIR*. This will allow you to keep custom configuration files separate which will make it easier
to fully remove TMSS without removing custom settings. 

## Installing TMSS
Once the prerequisite steps are taken and the required and optional environment variables are set, simply run the
installation script in the test mode:
```
cd $HOME/src/tms_server
./deployment/native/install_030.sh --test
```

You will be prompted to review and accept the detected settings before continuing. Once installation is complete,
output of the initialization run may be found in file `$TMS_ROOT_DIR/tms-install.out`. By default, this file
is located at `$HOME/.tms/tms-install.out`.

This output file contains the administrator credentials.
**WARNING This is only place where these credentials are displayed. Losing this information prevents administrative
actions and will likely make reinstallation necessary.**

The installation script will:
- Build TMSS from source files.
- Copy the SSL certificate files into place.
- Initialize the configuration by running `tms_server --install --root-dir $TMS_ROOT_DIR`.
- If needed copy custom `tms.toml` and `log4rs.yml` files from `$TMS_LOCAL_DIR`.

## Running TMSS
Note that the installation script will not start the service after installing. The server may be started up using
cargo. For example:
```
cd $HOME/src/tms_server
cargo run
```
Or started in the foreground by running the executable:
```
cd $HOME/src/tms_server
./target/release/tms_server
```

Note that a copy of the binary executable is also installed at `/tmp/tms_server/tms_server`

The server should now be available on the localhost at port 3000. For example, to fetch the current TMS Server
version:
```
curl -k https://localhost:3000/v1/tms/version
```

## Logging
The log configuration and formatting for `tms_server` is specified in the configuration file
`$TMS_ROOT_DIR/config/log4rs.yml`. By default, the log level is set to `INFO` and log messages are written to the file
`$TMS_ROOT_DIR/logs/tms_roller.log`.

## TMSS Directories and Files

In this section we list the directories and files that are part of a TMSS installation.

Defaults:
```
TMS_ROOT_DIR    : $HOME/.tms
TMS_LOCAL_DIR   : $HOME/tms_local
```

Under `$TMS_ROOT_DIR`
1. File *tms-db-env* - PostgreSQL DB settings. Used by backup script.
2. File *tms-install.out* - Output generated during installation, including administrator credentials.
3. File *tms_service.env* - Settings required when running `tms_server` as a service.
    * **WARNING This is only place where these credentials are displayed. Losing them will likely make reinstallation necessary.**
4. Directory **certs/** - Files:
     * *cert.pem* - Full-chain SSL certificate. Loaded at startup. In PEM format.
     * *key.pem* - Private key associated with the SSL certificate. In PEM format.
5. Directory **config** - Files:
    * *tms.toml* - The tms_server parameter file, which specifies the runtime options with which TMSS executes.
    * *log4rs.yml* - The log configuration and formatting for tms_server.
6. Directory **logs** - Default location of log files as configured in *log4rs.yml*.
7. Directory **migrations** - Files defining the DB schema.

Under `$TMS_LOCAL_DIR`
1. File *tms.toml* - (Optional) Local customizations of TMSS configuration settings.
2. File *log4rs.toml* - (Optional) Local customizations of TMSS log settings.

Under `$HOME/backups`
1. Directory **scripts/** - Files:
    * *backup_tms_server.sh* - Script to back up TMSS DB. Typically run as a cron job. Please see *backup/README.md*.
2. Directory **tms/** - Directory containing compressed backup files.
