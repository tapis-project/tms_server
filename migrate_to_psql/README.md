# TMS Server DB migrations from sqlite to postgres

This directory contains scripts used to migrate DB from sqlite to postgres.

The migration should be performed during the upgrade from TMS Server 0.2 to 0.3.

These files will not be needed during a clean installation of TMS Server 0.3 or later.

## Files
  - migrate_from_sqlite.sh - Main migration script
  - gres_r.sh - Supporting script providing in-place search and replace of strings in a file

## Setup
Certain environment variables must be set before running this script. The migrate script is run from
the upgrade script `upgrade_020_030.sh`.

The following environment variables must be set before running the migrations script:

  - TMS_INSTALL_DIR - Path to location of TMS server installation
  - TMS_DB_HOST - Postgresql server host
  - TMS_DB_PORT - Postgresql server port
  - TMS_DB_USER - Postgresql user
  - TMS_DB_PASSWORD - Postgresql user password 
  - POSTGRES_PASSWORD - Password for root user "postgres"

## Execution

Before running the migration script the TMS server service should be shut down and the environment variables set as
described above. Running the script will export data from the SQLite3 DB and then import data into the postgres DB.

The migration script performs the following steps:
1. Create a staging file and directory under /tmp
2. Export and post-process data from SQLite3 DB
3. Reset the postgres DB by dropping the TMS schema and re-creating it.
4. Import data into postgres DB.

For debugging purposes the final cleanup steps at the end of the script may be commented out allowing you to see
the temporary files.

