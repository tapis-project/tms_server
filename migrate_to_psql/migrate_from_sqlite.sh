#!/bin/bash
# Script to migrate data from an SQLite DB to a postgresql DB
#
# Script assumes SQLite and postgres psql are installed.
#
# Required env variables:
#   TMS_ROOT_DIR e.g. $HOME/.tms
#   TMS_INSTALL_DIR e.g. /opt/tms_server
#   TMS_DB_HOST     e.g. localhost
#   TMS_DB_PORT     e.g. 5431
#   TMS_DB_USER     e.g. tms
#   TMS_DB_USER_PASSWORD
#   POSTGRES_PASSWORD
#
# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

# Some operations are relative to the top level source directory.
SRC_DIR=$PRG_PATH/..

# This script should only be run when upgrading from TMS server version 0.2.0 to 0.3.0
VERS_OLD_REQUIRED="0.2.0"
VERS_NEW_REQUIRED="0.3.0"

# Check that all required env variables are set
FAILED=false
env_list="POSTGRES_PASSWORD TMS_ROOT_DIR TMS_INSTALL_DIR TMS_DB_HOST TMS_DB_PORT TMS_DB_USER TMS_DB_USER_PASSWORD TMS_TEST_MODE TMS_VERS_NEW"
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

# Make sure this is an upgrade from TMS Server 0.2.0 to 0.3.0
# Determine old and new versions
VERS_FILE=$TMS_INSTALL_DIR/tms.version
if [ ! -f "$VERS_FILE" ]; then
  echo "Unable to determine existing TMS version. Cannot find file: $VERS_FILE"
  echo "Exiting ..."
  exit 1
fi
VERS_OLD=$(cat $VERS_FILE)
if [ "$VERS_OLD" != "$VERS_OLD_REQUIRED" ] || [ "$TMS_VERS_NEW" != "$VERS_NEW_REQUIRED" ]; then
  echo "This script should only be run when upgrading from version 0.2.0 to 0.3.0."
  echo "Found VERS_OLD=$VERS_OLD and VERS_NEW=$TMS_VERS_NEW"
  echo "Exiting ..."
  exit 1
fi

# Create temporary staging file and directory
TMP_FILE=$(mktemp)
STG_DIR=/tmp/tms_migrate
mkdir -p $STG_DIR

# Path to SQLite3 DB file
TMS_SQ3_DB_PATH=$TMS_ROOT_DIR/database/tms.db

echo "**********************************************************************"
echo "   Exporting tables from SQLite DB: ${TMS_SQ3_DB_PATH}"
echo "**********************************************************************"
SQ3_CMD="sqlite3 ${TMS_SQ3_DB_PATH}"
table_list_all="admin clients delegations hosts pubkeys reservations tenants user_hosts user_mfa"
for t in $table_list_all
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
table_list_postprocess="clients tenants user_mfa"
for t in $table_list_postprocess
do
  out_file=${STG_DIR}/${t}.sql
  echo "Post-processing table: ${t}"
  $PRG_PATH/gres_r.sh "',1,'" "',1::bool,'" ${out_file}
  $PRG_PATH/gres_r.sh "',0,'" "',0::bool,'" ${out_file}
done

# Initialize the postgres DB
# TODO Should we check first that the tables do not yet exist?
#      Would it be bad if upgrade fails part way through and we re-run this and it re-imported some data? or would that fail?
echo "**********************************************************************"
echo "   Initializing Postgres DB for TMS"
echo "**********************************************************************"
$SRC_DIR/deployment/tms_init_db.sh
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "tms_init_db failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

echo "*********************************************************************************"
echo "   Creating TMS postgres DB schema"
echo "*********************************************************************************"
# Create the initial tms schema
#export TMS_DB_URL="postgres://${TMS_DB_USER}:${TMS_DB_USER_PASSWORD}@${TMS_DB_HOST}:${TMS_DB_PORT}/tmsdb"
if [ "$TEST_MODE" == "true" ]; then
  $SRC_DIR/target/release/tms_server --schema-only
  RET_CODE=$?
else
  su - $INSTALL_USR -c "$SRC_DIR/target/release/tms_server --schema-only"
  RET_CODE=$?
fi
if [ $RET_CODE -ne 0 ]; then
  echo "TMS server schema create failed"
  echo "Exiting ..."
  exit $RET_CODE
fi

echo "**********************************************************************"
echo "   Importing tables into postgresql DB"
echo "**********************************************************************"
# Put PGPASSWORD in environment for psql to pick up
export PGPASSWORD=${TMS_DB_USER_PASSWORD}
PSQL_CMD="psql -h ${TMS_DB_HOST} -p ${TMS_DB_PORT} -U ${TMS_DB_USER} -d tmsdb -q"

# List of tables to populate. Note: order is important due to foreign key constraints. Tenants first.
table_list_import="tenants clients user_mfa user_hosts admin delegations hosts reservations pubkeys"
for t in $table_list_import
do
  in_file=${STG_DIR}/${t}.sql
  echo "Importing data for table: ${t}"
  $PSQL_CMD -f ${in_file}
  RET_CODE=$?
  if [ $RET_CODE -ne 0 ]; then
    echo "TMS server schema migrate failed when importing sql for table: ${t}"
    echo "Exiting ..."
    exit $RET_CODE
  fi
done

echo "**********************************************************************"
echo "   Resetting table sequence IDs"
echo "**********************************************************************"
for t in $table_list_import
do
  echo "Resetting sequence ID for table: ${t}"
  # Build sql string for the update. E.g. select setval('tenants_id_seq',(SELECT MAX(id) FROM tenants));
  SQL_STR="SELECT setval('${t}_id_seq', (SELECT MAX(id) FROM ${t}));"
  $PSQL_CMD -c "$SQL_STR"
  RET_CODE=$?
  if [ $RET_CODE -ne 0 ]; then
    echo "TMS server schema migrate failed when resetting sequence ID for table: ${t}"
    echo "Exiting ..."
    exit $RET_CODE
  fi
done

# Move old sqlite database files to a backup directory
BAK_TIMESTAMP=`date  +%Y%m%d%H%M%S`
mv $TMS_ROOT_DIR/database $TMS_ROOT_DIR/database_bak_$BAK_TIMESTAMP

# Cleanup temporary staging file and directory
echo "**********************************************************************"
echo "   Final cleanup"
echo "**********************************************************************"
rm $TMP_FILE
rm $STG_DIR/*.sql
rmdir $STG_DIR
cd $RUN_DIR
