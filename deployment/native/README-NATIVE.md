# Installing and Upgrading the TMS Server

## Overview
The native installation of the TMS Server (TMSS) sets up the `tms_server` binary to run as a service that is managed
by `systemctl`. This file describes the installation, configuration and execution of the `tms_server` binary.

## One-Time Installation Procedures
Perform the following one-time installation steps prior to installing TMSS for the first time. These steps will not
be needed when upgrading.

### Install PostgreSQL
This may be installed almost anywhere. The simplest option is to install locally as a docker deployment.
Please see files under the directory `deployment/postgres` for an example docker compose file and scripts that may
be used to deploy a local postgres server. In order to use the scripts in this repo you will need to have the
postgres admin user as `postgres` and save the admin password for later use when initializing the DB for TMSS.

### Create user named tms
Create a user named `tms` on the linux where `tms_server` will run:
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
The database will be namde `tmsdb` and the schema (aka user) will be named `tms`. Note that if this is a re-install
and you wish to destructively remove a previous install you may use the script located at
`deployment/postgres/tms_drop.sh`. To initialize the DB you will need to choose a DB user password, set two environment
variables and run the init script, as follows:
```
su - tms
cd tms_server
export POSTGRES_PASSWORD=<pg_password>
export TMS_DB_USER_PASSWORD=<tms_user_password>
./deployment/tms_init_db.sh
```

If the PostgreSQL deployment is not running on `localhost` at port `5432` then the environment variables `TMS_DB_HOST`
and `TMS_DB_PORT` may be used to override the settings.

## Installing TMSS
When installing or upgrading TMSS you must be running as the root user. After the installation or upgrade  all operations
except for starting and stopping the service should be performed as the user `tms`.



# ????
1. Login as the **tms** user.
   1. Since the tms account has no password, issue "su - tms" from root or another sudoer account on the system. 
1. cd ~/tms_server/deployment/native
1. ./native_build_install_tms.sh
   1. This command will build TMSS from source files and create and/or populate these directories:
      1. **~/.tms** - The runtime configuration directory for tms-server.
      1. **~/tms_customizations** - The local customization directory preserved across native_build_install_tms.sh invocations.
      1. **/opt/tms_server** - The directory that contains the just built tms_server executable.
      1. **/opt/tms_server/lib/systemd/system/tms_server.service** - A systemd unit file that can be used to manage tms_server. 

## The ~/.tms Runtime Directory
After installation, tms_server uses the *~/.tms* directory during execution.  The directory contains these subdirectories.

1. **certs** contains these files:
   * *cert.pem* - The fullchain certificate file in PEM format that tms_server loads at startup.  A self-signed certificate file is shipped with TMSS, which is useful to get up and running for testing purposes, but is not meant for production.
   * *key.pem* - A private key that works with the self-signed certificate.
   * These two files should be replaced with a certificate/key combination generated for the TMSS host by a trusted certificate authority.  Below we discuss the *native_copy_certs.sh* utility program that helps with certificate and key management.

2. **config** contains these files:
   * *tms.toml* - The tms_server parameter file, which specifies the runtime options with which TMSS executes.
   * *log4rs.yml* - The log configuration and formatting for tms_server.

3. **database** contains the sqlite database files managed solely by tms_server.
4. **logs** contains the log files that were configured in *log4rs.yml*.
5. **migration** contains the database schema migration files.

## The ~/tms_customizations Directory
The *~/tms_customizations* directory contains information generated during installation and optional local customizations supplied by the TMSS administrator.
The files contained in this directory can include:
1. **cert.path** - An example pathname to a fullchain certificate file.  The *native_copy_certs.sh* utility program uses the path contained in this file to locate the source certificate file.
2. **key.path** - An example pathname to the private key file associated with the host's certificate.  The *native_copy_certs.sh* utility program uses the path contained in this file to locate the source key file.
3. **tms-install.out** - The installation program's output during new installations, including the *default* and *test* tenants' administrator credentials.  *This is only place where these credentials are displayed; losing this information prevents administrative actions in these two tenants and will likely make reinstallation necessary.* 
4. **Optional Files** - TMSS administrators can copy zero or more files from the *~/.tms/config* directory and modify them here.  The modified files are written to the *~/.tms/config* directory everytime *native_build_install_tms.sh* is invoked, whether on a new or existing installation.  This mechanism provides a convenient way to maintain a customized TMSS configuration across upgrades and reinstallations.  See the next section for details.

## native_build_install_tms.sh
Here's a more detailed explanation on how *native_build_install_tms.sh* sets up the runtime environment and how it can be leveraged.  There are basically two installation scenarios, new and existing, and different processing takes place in each case.

### Processing in New and Existing Installations
Whether TMSS is being installed for the first time on a host or if it already exists there, *native_build_install_tms.sh* always takes the following actions:

1. A cargo build is always attempted.  Any changes to the source code will trigger compilation.
2. The *tms_server* executable is always written to */opt/tms_server*.
3. The *tms_server.service* file is copied to */opt/tms_server/lib/systemd/system* if it doesn't already exist there.
4. The example *cert.pem* and *key.pem* files are written to *~/tms_customizations* if they don't already exist there.
5. If the *tms.toml* or *log4rs.yml* files exist in *~/tms_customizations*, they are written to *~/.tms/config*.

### Processing in New Installations Only
*native_build_install_tms.sh* detects a new installation based on whether the *~/.tms* directory exists. If that directory doesn't exist,
then the following actions are taken in addition to the ones listed in the previous section:

1. The *~/.tms* directory is created and populated with its default files.
2. A newly generated *tms-install.out* file is written to *~/tms_customizations*, overwriting a previously generated file if one exists. 

To replace an existing installation with a new one, simply delete the *~/.tms* directory subtree.  You can optionally remove all, some or none of the
*~/tms_customizations* content.

## native_copy_certs.sh
The *native_copy_certs.sh* utility program can be used to copy the host fullchain certificate and private key to the *~/.tms/certs/cert.pem* and
*~/.tms/certs/key.pem* files, respectively.  Here's what *native_copy_certs.sh* does:

1. Copies two files.
   1. It uses the *~/tms_customizations/cert.path* content as the source file path.  It copies the file at that source path to *~/.tms/certs/cert.pem*.
   1. It uses the *~/tms_customizations/key.path* content as the source file path.  It copies the file at that source path to *~/.tms/certs/key.pem*.
2. Changes r/w access to owner-only (600) on both copied files.
3. Changes owner:group of both copied files to tms:tms.

TMSS administrators can use any method they like to accomplish the same tasks; *native_copy_certs.sh* does not have to be used as long as the end state
regarding file placement, permissions and ownership is the same.

An important consideration for administrators is how to manage certificate/key expiration.  We assume some administrative process external to TMSS replaces
the host's certificate and key before they expire.  Ideally, this event will trigger the TMSS certificate and key file processing just described and then restart TMSS (see below). 

# Running TMSS
A convenient way to run TMSS is via systemctl.  The *tms_server.service* file that is written to the
*/opt/tms_server/lib/systemd/system* directory can be used as is or as a starting point for a systemd unit definition.
This file (or its derivative) can be copied to /etc/systemd/system or referenced in place using a symbolic link.
Either way, issuing *systemctl start tms_server* as root will start TMSS.  If TMSS is already running, then
*systemctl restart tms_server* should be used, such as after a new host certificate has been installed.

# Reinstalling TMSS

Once TMSS is installed and running on a machine, reinstalling entails rebuilding the latest code and copying the executable to the /opt/tms_server directory.
If you launch TMSS using systemctl, you'll need to stop the service, copy the executable and then restart the service.

To build the latest code from the ~/tms_server directory, issue:

   - *git pull*
   - *cargo build --release*

To copy the new executable to /opt/tms_server, first stop *tms_server*, issue the command below, and then restart *tms_server*:    

   - *cp -p target/release/tms_server /opt/tms_server/*


The log for the service may be monitored using journalctl (running as root), for example:

`sudo journalctl -u tms_server.service -n 1000 -b -f
`
