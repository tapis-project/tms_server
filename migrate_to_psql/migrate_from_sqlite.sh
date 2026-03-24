#!/bin/bash
#
# Script to migrate data from an SQLite DB to a postgresql DB
# Script assumes SQLite is installed.
# Required env variables:
#   TMS_SQ3_DB_PATH e.g. ~/.tms/database/tms.db
#   TMS_DB_HOST     e.g. localhost
#   TMS_DB_PORT     e.g. 5431
#   TMS_DB_PASSWORD
#
# Determine absolute path to location from which we are running and change to that directory.
export RUN_DIR=$(pwd)
export PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
export PRG_PATH=$(pwd)

FAILED=false
env_list="TMS_SQ3_DB_PATH TMS_DB_HOST TMS_DB_PORT TMS_DB_PASSWORD"
for name in $env_list
do
  if [[ -z "${!name}" ]]; then
    echo "Please set env var ${name} before running this script"
    FAILED=true
  fi
done
if [ "$FAILED" = true ]; then
  echo "Please set required environment variables"
  echo "Exiting ..."
  exit 1
fi

export SQ3_CMD="sqlite3 ${TMS_SQ3_DB_PATH}"
# TODO
export PSQL_CMD="TODO"

TMP_FILE=$(mktemp)
echo "**********************************************************************"
echo "   Exporting tables from SQLite DB: ${TMS_SQ3_DB_PATH}"
echo "**********************************************************************"
table_list="admin clients delegations hosts pubkeys reservations tenants user_hosts user_mfa"
for t in $table_list
do
  echo "Migrating table: ${t}"
  # Generate the sql inserts. Put export commands in tmp file
  echo ".mode insert" > $TMP_FILE
  echo ".output ${t}.sql" >> $TMP_FILE
  echo "select * from ${t};" >> $TMP_FILE
  $SQ3_CMD ".read $TMP_FILE"
  # TODO Post-process the sql command. Replace INSERT INTO "table" with correct table name
  $PRG_PATH/gres_r.sh "INSERT INTO \"table\"" "INSERT INTO \"${t}\"" ${t}.sql
done

echo "**********************************************************************"
echo "   Post-processing sql files."
echo "**********************************************************************"
# Bool type not used in sqlite. Cast to bool in postgresql
table_list="clients tenants user_mfa"
for t in $table_list
do
  echo "Post-processing table: ${t}"
  # Post-process the sql command. Replace INSERT INTO "table" with correct table name
  $PRG_PATH/gres_r.sh "',1,'" "',1::bool,'" ${t}.sql
  $PRG_PATH/gres_r.sh "',0,'" "',0::bool,'" ${t}.sql
done

# TODO
echo "**********************************************************************"
echo "   Importing tables into postgresql DB"
echo "**********************************************************************"

rm $TMP_FILE
