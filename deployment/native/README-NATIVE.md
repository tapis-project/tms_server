# Overview
The native installation of tms_server runs the tms_server binary, typically started via systemctl.  This file describes the installation, configuration and execution of the tms_server binary.

# One-Time Installation Procedures
Perform the following steps on the host that will run tms_server.

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

5. Clone the tms_server repository in the tms home directory.

   1. git clone https://github.com/tapis-project/tms_server.git

# Install TMS
1. Login as the tms user.
   1. Since the tms account has no password, issue "su - tms" from root or another sudoer account on the system. 
1. cd ~/tms_server/deployment/native
1. ./native_build_install_tms.sh
   1. This command will build TMS from source files and create and/or populate these directories:
      1. **~/.tms** - The runtime configuration directory for tms-server.
      1. **~/tms_customizations** - The local customization directory preserved across native_build_install_tms.sh invocations.
      1. **/opt/tms_server** - The directory that contains the just built tms_server executable.
      1. **/opt/tms_server/lib/systemd/systems/tms_server.service** - A systemd unit file that can be used to manage tms_server. 

## The ~/.tms Runtime Directory
The ~/.tms directory contains these subdirectories, which are used by tms_server while it executes.
1. **certs** contains these files:
   * *cert.pem* - The fullchain certificate file in PEM format that tms_server loads at startup.  A self-signed certificate file is shipped with TMS, which is useful to get up and running for testing purposes, but is not meant for production.
   * *key.pem* - A private key that works with the self-signed certificate.

These files should be replaced with a certificate/key combination that was generated for the TMS host by a trusted certificate authority.  Below we discuss the *native_copy_certs.sh* utility program that helps with certificate and key management.

2. **config** contains these files:
   * *tms.toml* - The tms_server parameter file, which specifies the options with which TMS executes.
   * *log4rs.yml* - The log configuration and formatting for tms_server.

3. **database** contains the sqlite database files managed solely by tms_server.
4. **logs** contains the log files that were configured in *log4rs.yml*.
5. **migration** contains the database schema migration files.

## The ~/tms_customizations Directory
The ~/tms_customizations directory contains information generated during installation and optional local customizations supplied by the TMS administrator.  The files contained in this directory include:
1. **cert.path** - An example pathname to a fullchain certificate file.  The *native_copy_certs.sh* utility program uses the path contained in this file to locate the source certificate file.
2. **key.path** - An example pathname to the private key file associated with the host's certificate.  The *native_copy_certs.sh* utility program uses the path contained in this file to locate the source key file.
3. **tms-install.out** - The installation program's output during new installations, including the *default* and *test* tenants' administrator credentials.  *This is only place where these credentials are displayed; losing this information prevents administrative actions in these two tenants and will likely make a reinstallation necessary.* 
4. **Optional Files** - TMS administrators can copy zero or more files from the *~/.tms/config* directory and modify them here.  The modified files are written to the *~/.tms/config* directory everytime *native_build_install_tms.sh* is invoked, providing a safe, convenient way to reconfigure TMS without unexpectedly having customizations overwritten.

## native_build_install_tms.sh
In this section, we provide more detail on how *native_build_install_tms.sh* sets up the runtime TMS environment and how that knowledge can be used to manage TMS.  There are two installation scenarios, new and existing, and different processing takes place in each case.

### Processing in New and Existing Installations
Whether TMS is being installed for the first time on a host or if it already exists there, *native_build_install_tms.sh* always takes the following actions:

1. A cargo build is always attempted.  Any changes to the source code will trigger compilation.
2. The *tms_server* executable is always written to */opt/tms_server*.
3. The *tms_server.service* file is copied to */opt/tms_server/lib/systemd/systems* if it doesn't already exist there.
4. The example *cert.pem* and *key.pem* files are written to *~/tms_customizations* if they don't already exist there.
5. If the *tms.toml* or *log4rs.yml* files exist in *~/tms_customizations*, they are written to *~/.tms/config*.

### Processing in New Installations Only
*native_build_install_tms.sh* detects a new installation based on whether the *~/.tms* directory exists.  If the directory doesn't exist, then the following actions are taken in addition to the ones listed in the previous section:

1. The *~/.tms* directory is created and populated with its default files.
2. The *tms-install.out* file is written to *~/tms_customizations*, overwriting a previously generated file if it exists. 

To replace an existing installation with a new one, simply delete the *~/.tms* directory subtree.  You can optionally remove all, some or none of the *~/tms_customizations* content.
