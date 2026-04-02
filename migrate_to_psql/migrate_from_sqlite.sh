#!/bin/bash
#
# Script to migrate data from an SQLite DB to a postgresql DB
# Script assumes SQLite is installed.
# Required env variables:
#   TMS_INSTALL_DIR e.g. ~/.tms
#   TMS_DB_HOST     e.g. localhost
#   TMS_DB_PORT     e.g. 5431
#   TMS_DB_USER     e.g. tms
#   TMS_DB_PASSWORD
#
# Determine absolute path to location from which we are running and change to that directory.
export RUN_DIR=$(pwd)
export PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
export PRG_PATH=$(pwd)

# This script should only be run when upgrading from TMS server version 0.2.0
VERS_OLD_REQUIRED="0.2.0"

# Check that all required env variables are set
FAILED=false
env_list="TMS_INSTALL_DIR TMS_DB_HOST TMS_DB_PORT TMS_DB_USER TMS_DB_PASSWORD"
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

# Path to SQLite3 DB file
TMS_SQ3_DB_PATH=$TMS_INSTALL_DIR/database/tms.db

# Make sure this is an upgrade from TMS Server 0.2.0
# Determine existing version
VERS_FILE=$TMS_INSTALL_DIR/tms.version
if [ ! -f "$VERS_FILE" ]; then
  echo "Unable to determine existing TMS version. Cannot find file: $VERS_FILE"
  echo "Exiting ..."
  exit 1
fi
VERS_OLD=$(cat $VERS_FILE)
if [ "$VERS_OLD" != "$VERS_OLD_REQUIRED" ]; then
  echo "This script should only be run when upgrading form version 0.2.0. Found version: $VERS_OLD"
  echo "Exiting ..."
  exit 1
fi

SQ3_CMD="sqlite3 ${TMS_SQ3_DB_PATH}"

# Create temporary staging file and directory
TMP_FILE=$(mktemp)
STG_DIR=/tmp/tms_migrate
mkdir -p $STG_DIR

echo "**********************************************************************"
echo "   Exporting tables from SQLite DB: ${TMS_SQ3_DB_PATH}"
echo "**********************************************************************"
table_list="admin clients delegations hosts pubkeys reservations tenants user_hosts user_mfa"
for t in $table_list
do
  echo "Migrating table: ${t}"
  out_file=${STG_DIR}/${t}.sql
  # Generate the sql inserts. Put export commands in tmp file
  echo ".mode insert" > $TMP_FILE
  echo ".output ${out_file}" >> $TMP_FILE
  echo "select * from ${t};" >> $TMP_FILE
  $SQ3_CMD ".read $TMP_FILE"
  # Post-process the sql command. Replace INSERT INTO "table" with correct table name
  $PRG_PATH/gres_r.sh "INSERT INTO \"table\"" "INSERT INTO \"${t}\"" ${out_file}
done

echo "**********************************************************************"
echo "   Post-processing sql files."
echo "**********************************************************************"
# Bool type not used in sqlite. Cast to bool in postgresql
table_list="clients tenants user_mfa"
for t in $table_list
do
  out_file=${STG_DIR}/${t}.sql
  echo "Post-processing table: ${t}"
  $PRG_PATH/gres_r.sh "',1,'" "',1::bool,'" ${out_file}
  $PRG_PATH/gres_r.sh "',0,'" "',0::bool,'" ${out_file}
done

# TODO
echo "**********************************************************************"
echo "   TODO Importing tables into postgresql DB"
echo "**********************************************************************"

PSQL_CMD="TODO"

# TODO Cleanup temporary staging file and directory
#rm $TMP_FILE
#rm $STG_DIR/*.sql
#rmdir $STG_DIR