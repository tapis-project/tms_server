# Overview
The native installation of TMS runs the *tms_server* binary, which is usually started via systemctl.  This file describes the installation, configuration and execution of the tms_server binary.

# One-Time Installation Procedures
Perform the following one-time installation steps on the host that will run tms_server.

1. Create tms linux user.

   1. As root:  adduser --disabled-password tms

2. Create /opt/tms_server.  As root:

   1. cd /opt

   2. mkdir tms_server

   3. chown tms:tms tms

   4. chmod 750 tms_server

3. su - tms

4. Install rust

   1. curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   2. In a new terminal, issue: rustc --version

   3. The latest stable version greater than or equal to 1.82.0 should be displayed.

   4. *Future maintenance may require using rustup to upgrade rust.* 

5. Clone the tms_server repository in the tms home directory.

   1. git clone https://github.com/tapis-project/tms_server.git
  

# Installing TMS
1. Login as the **tms** user.
   1. Since the tms account has no password, issue "su - tms" from root or another sudoer account on the system. 
1. cd ~/tms_server/deployment/native
1. ./native_build_install_tms.sh
   1. This command will build TMS from source files and create and/or populate these directories:
      1. **~/.tms** - The runtime configuration directory for tms-server.
      1. **~/tms_customizations** - The local customization directory preserved across native_build_install_tms.sh invocations.
      1. **/opt/tms_server** - The directory that contains the just built tms_server executable.
      1. **/opt/tms_server/lib/systemd/system/tms_server.service** - A systemd unit file that can be used to manage tms_server. 

## The ~/.tms Runtime Directory
After installation, tms_server uses the *~/.tms* directory during execution.  The directory contains these subdirectories.

1. **certs** contains these files:
   * *cert.pem* - The fullchain certificate file in PEM format that tms_server loads at startup.  A self-signed certificate file is shipped with TMS, which is useful to get up and running for testing purposes, but is not meant for production.
   * *key.pem* - A private key that works with the self-signed certificate.
   * These two files should be replaced with a certificate/key combination generated for the TMS host by a trusted certificate authority.  Below we discuss the *native_copy_certs.sh* utility program that helps with certificate and key management.

2. **config** contains these files:
   * *tms.toml* - The tms_server parameter file, which specifies the runtime options with which TMS executes.
   * *log4rs.yml* - The log configuration and formatting for tms_server.

3. **database** contains the sqlite database files managed solely by tms_server.
4. **logs** contains the log files that were configured in *log4rs.yml*.
5. **migration** contains the database schema migration files.

## The ~/tms_customizations Directory
The *~/tms_customizations* directory contains information generated during installation and optional local customizations supplied by the TMS administrator.  The files contained in this directory can include:
1. **cert.path** - An example pathname to a fullchain certificate file.  The *native_copy_certs.sh* utility program uses the path contained in this file to locate the source certificate file.
2. **key.path** - An example pathname to the private key file associated with the host's certificate.  The *native_copy_certs.sh* utility program uses the path contained in this file to locate the source key file.
3. **tms-install.out** - The installation program's output during new installations, including the *default* and *test* tenants' administrator credentials.  *This is only place where these credentials are displayed; losing this information prevents administrative actions in these two tenants and will likely make reinstallation necessary.* 
4. **Optional Files** - TMS administrators can copy zero or more files from the *~/.tms/config* directory and modify them here.  The modified files are written to the *~/.tms/config* directory everytime *native_build_install_tms.sh* is invoked, whether on a new or existing installation.  This mechanism provides a convenient way to maintain a customized TMS configuration across upgrades and reinstallations.  See the next section for details.

## native_build_install_tms.sh
Here's a more detailed explanation on how *native_build_install_tms.sh* sets up the runtime environment and how it can be leveraged.  There are basically two installation scenarios, new and existing, and different processing takes place in each case.

### Processing in New and Existing Installations
Whether TMS is being installed for the first time on a host or if it already exists there, *native_build_install_tms.sh* always takes the following actions:

1. A cargo build is always attempted.  Any changes to the source code will trigger compilation.
2. The *tms_server* executable is always written to */opt/tms_server*.
3. The *tms_server.service* file is copied to */opt/tms_server/lib/systemd/system* if it doesn't already exist there.
4. The example *cert.pem* and *key.pem* files are written to *~/tms_customizations* if they don't already exist there.
5. If the *tms.toml* or *log4rs.yml* files exist in *~/tms_customizations*, they are written to *~/.tms/config*.

### Processing in New Installations Only
*native_build_install_tms.sh* detects a new installation based on whether the *~/.tms* directory exists.  If that directory doesn't exist, then the following actions are taken in addition to the ones listed in the previous section:

1. The *~/.tms* directory is created and populated with its default files.
2. A newly generated *tms-install.out* file is written to *~/tms_customizations*, overwriting a previously generated file if one exists. 

To replace an existing installation with a new one, simply delete the *~/.tms* directory subtree.  You can optionally remove all, some or none of the *~/tms_customizations* content.

## native_copy_certs.sh
The *native_copy_certs.sh* utility program can be used to copy the host fullchain certificate and private key to the *~/.tms/certs/cert.pem* and *~/.tms/certs/key.pem* files, respectively.  Here's what *native_copy_certs.sh* does:

1. Copies two files.
   1. It uses the *~/tms_customizations/cert.path content as the source file path.  It copies the file at that source path to *~/.tms/certs/cert.pem*.
   1. It uses the *~/tms_customizations/key.path content as the source file path.  It copies the file at that source path to *~/.tms/certs/key.pem*.
2. Changes r/w access to owner-only (600) on both copied files.
3. Changes owner:group of both copied files to tms:tms.

TMS administrators can use any method they like to accomplish the same tasks; *native_copy_certs.sh* does not have to be used as long as the end state regarding file placement, permissions and ownership is the same.

An important consideration for administrators is how to manage certificate/key expiration.  We assume some administrative process external to TMS replaces the host's certificate and key before they expire.  Ideally, this event will trigger the TMS certificate and key file processing just described and then restart TMS (see below). 

# Running TMS
A convenient way to run TMS is via systemctl.  The *tms_server.service* file that is written to the */opt/tms_server/lib/systemd/system* directory can be used as is or as a starting point for a systemd unit definition.  This file (or its derivative) can be copied to /etc/systemd/system or referenced in place using a symbolic link.  Either way, issuing *systemctl start tms_server* as root will start TMS.  If TMS is already running, then *systemctl restart tms_server* should be used, such as after a new host certificate has been installed.

