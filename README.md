# tms_server

Trust Manager System (TMS) web server

TMS is currently in prototype form and is being developed for a Minimal Viable Product release.  As development proceeds the SQLite database schema may change.  To upgrade to the new schema delete all tms.db* files from the top-level directory (all content will be lost).  TMS will automatically create a new database with the updated schema when run.

## Running the TMS Server

To run the server from the command line, type "cargo run".

To run the server from within Visual Studio, press the *Run* hotspot above the main() function in main.rs.

The automatically generated livedocs can be accessed by pointing your browser to https://localhost:3000.  The various APIs can be executed by filling in form data.

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

The following algorithms can be specifed in the *key_type*, with the default being RSA (4096):

- **RSA**
- **ECDSA**
- **ED25519**


 
