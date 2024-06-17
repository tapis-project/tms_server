# tms_server

Trust Manager System (TMS) web server

TMS is currently being developed for a Minimal Viable Product release.  As development proceeds the SQLite database schema may change.  To upgrade to the new schema delete all tms.db* files from the *database* directory under the TMS root directory (all content will be lost).  TMS will automatically create a new database with the updated schema when run.

## Running the TMS Server (TMSS)

There are 3 ways to start TMSS:

  1. To run the server from the command line in a development environment, type "cargo run" from the top-level *tms_server* directory.

  2. To run the server from within Visual Studio, press the *Run* hotspot above the main() function in main.rs.

  3. To run the server from the directory in which the server's executable resides, such as *target/debug*, issue "./tms_server".  
  
  Note that the current directory from which TMSS is lauched must have a **resources** subdirectory.  The resources directory must match the resources directory in the source code repository, including all subdirectories and files.  This requirement is automatically met by launch methods 1 and 2 above, but usually requires a manual or automated process to copy content from the source code repository for method 3. 

Use the --help flag to see the supported command line options.

The automatically generated livedocs can be accessed by pointing your browser to https://localhost:3000.  The various APIs can be executed by filling in form data.

## TMS Root Data Directory

TMSS expects certain externals files to exist in a well-defined directory structure.  By default, TMSS will create its directory subtree rooted at **~/.tms**.  The root directory has the following subdirectories:

  - ~/.tms
      - certs
      - config
      - database
      - keygen
      - logs
      - migrations

## Initializing the TMS Root Data Directory

The TMSS automatically initializes its root data directory on startup.  Initialization includes creating the root directory and all its subdirectories as well as moving the default configuration files into those subdirectories.  These default files are:

  - ~/.tms/certs/cert.pem

  - ~/.tms/certs/key.pem

  - ~/.tms/config/log4rs.yml

  - ~/.tms/config/tms.toml

  - ~/.tms/migrations/20240315211037_tms_init.sql

TMSS sets owner-only permissions on it directories and files.  The files in the **config** subdirectory can be modified for development or production purposes.  The **log4rs.yml** controls logging to the console and to rolling log files in the **~/.tms/logs** directory.  See **log4rs.yml** for instructions on changing the output directory for the rolling log.

The **tms.toml** file specifies the server's options that can be customized at runtime.  At least one server url should be specified so that the generated livedocs can execute commands (localhost is convenient).

## Customizing the Configuration

One can instruct TMSS to initialize its root directory and immediately exit.  This allows the configuration files to be customized before TMS starts handling requests.  Issue the following command to create and populate the TMS root directory and then exit:

  - ./tms_server --init-dirs-only

At this point, one can edit the various configuration files to meet his or her needs.  

### Non-Default Root Directory Initialization

  - Issue: ./tms_server --root-dir MYDIR

The --root-dir option can work in conjunction with the --init-dirs-only option.

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

 
