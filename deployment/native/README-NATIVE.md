# Installing and Upgrading the TMS Server version 0.4

## Overview
A native installation of the TMS Server (TMSS) sets up the `tms_server` binary to run as a service that is managed
by `systemctl`. This file describes the installation, configuration and execution of the `tms_server` binary.

## One-Time Installation Prerequisite Procedures
Perform the following one-time installation steps prior to installing TMSS for the first time.

### Install PostgreSQL
This may be installed and running almost anywhere. The simplest option is to install locally as a docker deployment.
Please see files under the directory `deployment/postgres` for an example docker compose file and scripts that may
be used to deploy a local postgres server. In order to use the scripts in this repo you will need to have the
postgres admin user as `postgres` and save the admin password for later use when initializing the DB for TMSS.

### Create user named tms
Create a user named `tms` on the linux host where `tms_server` will run:
```
useradd -m tms
```
### As user `tms` install rust and clone the repository
```
su - tms
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
git clone https://github.com/tapis-project/tms_server.git
```

### As user `tms` initialize the PostgreSQL database
Prior to installing TMSS Server for the first time the DB must be initialized by creating the database and schema.
The database will be named `tmsdb` and the schema (aka user) will be named `tms`. Note that if this is a re-installation
and you wish to destructively remove a previous install you may use the script located at
`deployment/postgres/tms_drop.sh`. To initialize the DB you will need to choose a DB user password, set two environment
variables and run the init script, as follows:
```
su - tms
cd tms_server
export POSTGRES_PASSWORD=<pg_password>
export TMS_DB_USER_PASSWORD=<tms_user_password>
./deployment/postgres/tms_init_db.sh
```

If the PostgreSQL deployment is not running on `localhost` at port `5432`, then the environment variables `TMS_DB_HOST`
and `TMS_DB_PORT` may be used to override the settings.

## Environment variables used during installation and upgrade

### Required
The following environment variables are required when installing TMSS:
- POSTGRES_PASSWORD
  - Password for the postgresql admin user `postgres`
- TMS_DB_USER_PASSWORD
  - Password for the TMS DB user
- TMS_SSL_CERT_PATH
  - Path to the SSL full-chain certificate file in PEM format that is loaded at startup.
- TMS_SSL_KEY_PATH
  - Path to the private key file in PEM format associated with the server SSL certificate.

### Optional
Other env variables that can be set to override defaults:
- TMS_DB_HOST
  - Host server running PostgreSQL. default = localhost
- TMS_DB_PORT
  - Port at which PostgreSQL server is running. default = 5432 :

Other less common env variable overrides:
- TMS_ROOT_DIR
  - Location of `certs/`, `config/`, `logs/`, `migrations/`. default = $HOME/.tms
- TMS_INSTALL_DIR
  - Location of `tms_server` executable, `tms.version` and `lib/`. default = /opt/tms_server
- TMS_LOCAL_DIR
  - Location of install output and optional custom `tms.toml`, `log4rs.yml`. default = $TMS_ROOT_DIR/local

NOTE: It is strongly recommended that *TMS_LOCAL_DIR* be left as the default or set to a directory
outside of *TMS_ROOT_DIR*. This will allow you to keep custom configuration files separate which will make it easier
to fully remove TMSS without removing custom settings.

## Installing TMSS
When installing or upgrading TMSS you must be running as the root user. After the installation or upgrade, all operations
except for starting and stopping the service should be performed as the user `tms`.

Once the prerequisite steps are taken and the required and optional environment variables are set, simply run the
installation script as root:
```
sudo su -
cd ~tms/tms_server
./deployment/native/install_030.sh
```

You will be prompted to review and accept the detected settings before continuing. Once installation is complete,
output of the initialization run may be found in file `$TMS_ROOT_DIR/tms-install.out`. By default, this file
is located at `~tms/.tms/tms-install.out`.

This output file contains credentials for the default admin user `~~admin`. 
**WARNING This is only place where these credentials are displayed. Losing this information prevents administrative
actions and will likely make reinstallation necessary.**

The installation script will:
- Create and update ownership of various directories and files, such as `$TMS_ROOT_DIR`, `$TMS_INSTALL_DIR`, etc.
- Build TMSS from source files and copy the executable into place.
- Copy the SSL certificate files into place.
- Initialize the configuration by running `tms_server --install --root-dir $TMS_ROOT_DIR`.
- If needed copy custom `tms.toml` and `log4rs.yml` files from `$TMS_LOCAL_DIR`.

## Upgrading TMSS from version `0.3` to version `0.4`
When installing or upgrading TMSS you must be running as the root user. After the installation or upgrade  all operations
except for starting and stopping the service should be performed as the user `tms`.

### Run the installation script with upgrade option
Once the prerequisite steps are taken and the required and optional environment variables are set, simply run the
installation script as root specifying the option `--upgrade`:
```
sudo su -
cd ~tms/tms_server
./deployment/native/install_030.sh --upgrade
```

You will be prompted to review and accept the detected settings before continuing.

The upgrade script will:
- Create and update ownership of any new directories or files that are included as part of the upgrade.
- Build TMSS from source files and copy the updated executable into place.
- Stop the TMSS service using `systemctl`.
- The existing `$TMS_ROOT_DIR/migrations` directory will be moved to a backup directory.
- The new `migrations` directory will be copied into place from `~tms/tms_server/resources/migrations`.

Please note that if upgrading the script will not overwrite the `tms.toml` or `log4rs.yml` files located in
`$TMS_ROOT_DIR/config`. Any customizations will remain in place.

## Running TMSS
Note that the installation script will not start the service after installing or upgrading.  

A convenient way to run TMSS is via `systemctl`. The installation script places a service configuration file
at `$TMS_INSTALL_DIR/lib/systemd/system/tms_server.service` which provides a starting point for a systemd unit
definition. The configuration may be used as-is. This file (or its derivative) can be copied to `/etc/systemd/system` or
referenced in place using a symbolic link. Here is an example of a command that can be run as root to create a
symbolic link:
```
sudo su -
ln -s /opt/tms_server/lib/systemd/system/tms_server.service /etc/systemd/system/tms_server.service
```

Note that the specific configuration may vary based on the host OS setup.

Once the service is configured the following commands may then be used to manage and monitor the service:
```
systemctl start tms_server.service
systemctl stop tms_server.service
systemctl status tms_server.service
journalctl -u tms_server.service -n 500 -b -f
```

The service configuration file `tms_server.service` has an entry that points to `$TMS_ROOT_DIR/tms_service.env` in
order to set up environment variables for the service.

## Logging
The log configuration and formatting for `tms_server` is specified in the configuration file
`$TMS_ROOT_DIR/config/log4rs.yml`. By default, the log level is set to `INFO` and log messages are written to the file
`$TMS_ROOT_DIR/logs/tms_roller.log`.

## Managing SSL Certificate
An important consideration for administrators is how to manage certificate expiration. We assume some administrative
process external to TMSS replaces the certificate and key before they expire. Ideally, this event will trigger the
TMSS certificate and key file processing and then restart TMSS.

## TMSS Directories and Files

In this section we list the directories and files that are part of a TMSS installation.

Defaults:
```
TMS_ROOT_DIR    : ~tms/.tms
TMS_LOCAL_DIR   : ~tms/tms_local
TMS_INSTALL_DIR : /opt/tms_server 
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

Under `~tms/backups`
1. Directory **scripts/** - Files:
    * *backup_tms_server.sh* - Script to back up TMSS DB. Typically run as a cron job. Please see *backup/README.md*.
2. Directory **tms/** - Directory containing compressed backup files.
