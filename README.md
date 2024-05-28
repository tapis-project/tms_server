# tms_server

Trust Manager System (TMS) web server

TMS is currently in prototype form and is being developed for a Minimal Viable Product release.  As development proceeds the SQLite database schema may change.  To upgrade to the new schema delete all tms.db* files from the top-level directory (all content will be lost).  TMS will automatically create a new database with the updated schema when run.

## Running the TMS Server (TMSS)

There are 3 ways to start TMSS:

  1. To run the server from the command line in a development environment, type "cargo run".

  2. To run the server from within Visual Studio, press the *Run* hotspot above the main() function in main.rs.

  3. To run the server from the directory in which the server's executable resides, such as *target/debug*, issue "./tms_server".

Use the --help flag to see the supported command line options.

The automatically generated livedocs can be accessed by pointing your browser to https://localhost:3000.  The various APIs can be executed by filling in form data.

## TMS Root Data Directory

TMSS expects certain externals files to exist in a well-defined directory structure.  By default, TMSS will create its directory subtree rooted at ~/.tms.  The root directory has the following subdirectories:

  - ~/.tms
      - certs
      - config
      - database
      - keygen
      - logs
      - migrations

## Initializing the TMS Root Data Directory

Initializing the root data directories is a 2 step process.  The first step creates the root directory and its subdirectories and the second step populates the subtree with required configuration and security files.  In the examples below, we invoke TMSS directly from the command line.  

### Step 1: Generating the Directories

Issue the following command to create the root directory and it subdirectories with the required permissions.

  - ./tms_server --create-dirs-only

### Step 2: Populating the Directories.

Starting at the tms_server source tree, issue the following commands:

  - cd resources

  - ./install-resources ~/.tms

The result is that these files will now be present under the root directory:

  - ~/.tms/certs/cert.pem

  - ~/.tms/certs/key.pem

  - ~/.tms/config/log4rs.yml

  - ~/.tms/config/tms.toml

  - ~/.tms/migrations/20240315211037_tms_init.sql

The files in the **config** subdirectory can be modified for development or production purposes.  The **log4rs.yml** controls logging to the console and to rolling log files in the **~/.tms/logs** directory.  See **log4rs.yml** for instructions on changing the output directory for the rolloing log.

The **tms.toml** file specifies the server's runtime options that can be customized at runtime.

## Using a Non-Default Root Data Directory

TMSS supports using a directory other than the default ~/.tms directory as its root directory.  Both the initialization and runtime execution processes need to be parameterized to support non-default root directories.  We use MYDIR as a custom root directory in the descriptions below.

### Non-Default Root Directory Initialization

  - Issue: ./tms_server --create-dirs-only --root-dir MYDIR

  - Issue: ./install-resources MYDIR

### Non-Default Root Directory Execution

At runtime TMSS needs to be pointed at MYDIR to use it as its root directory.  This can be done in two ways.

  1. Set the TMS_ROOT_DIR environment variable to MYDIR, or

  2. Start the server with a command line argument:  ./tms_server --root-dir MYDIR

If both methods are used, the environment variable setting takes priority.

## Tenancy 

When TMS initializes its database it creates two tenants: *default* and *test*.  The former is the standard TMS tenant used in production; the latter is a tenant for use in development.   

## The Test Tenant

As part of database initialization, TMS populates the *test* tenant with test data useful for running the */creds/sshkeys* and */creds/publickey* APIs.  This preloaded test data includes records in the clients, user_mfa, user_hosts and delgations tables.  These records establish the following entities:

- **Client ID:** testclient1
- **Client Secret:** secret1
- **User:** testuser1
- **Host:** testhost1
- **Host Account:** testhostaccount1
- **Application:** testapp1
- **Application Version:** 1.0

## Testing Public Key Creation

One can create test keys using the livedocs interface in a browser (https://localhost:3000).  Select the */creds/sshkeys* API and submit this input:

```
{
  "client_id": "testclient1",
  "client_secret": "secret1",
  "tenant": "test",
  "client_user_id": "testuser1",
  "host": "testhost1",
  "host_account": "testhostaccount1",
  "num_uses": 0,
  "ttl_minutes": 0,
  "key_type": ""
}        
```

The following algorithms can be specifed in the *key_type*, with the default being ED25519:

- **RSA**
- **ECDSA**
- **ED25519**

 
