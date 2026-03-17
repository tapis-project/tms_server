#!/bin/bash
# Script to migrate data from SQlite DB file located at ~/.tms/database/tms.db
# Postgres password must be set in env var POSTGRES_PASSWORD
#
# Determine absolute path to location from which we are running
#  and change to that directory.
export RUN_DIR=$(pwd)
export PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
export PRG_PATH=$(pwd)

if [ -z "$TMS_HOME" ]; then
  TMS_HOME=~/.tms
fi

if [ -z "$TMS_DB_HOST" ]; then
  TMS_DB_HOST=localohst
fi

if [ -z "$TMS_DB_PORT" ]; then
  TMS_DB_PORT=5432
fi

PSQL_CMD="psql --host=${TMS_DB_HOST} --port=${TMS_DB_PORT}"

DB_USER=postgres
DB_NAME=tmsdb
export TMS_DB_USER=tms
export TMS_DB_SCHEMA=tms

if [ -z "${POSTGRES_PASSWORD}" ]; then
  echo "Please set env var POSTGRES_PASSWORD before running this script"
  exit 1
fi

if ! command -v pgloader &> /dev/null; then
  echo "Please install pgloader utility before running this script"
  exit 1
fi

# Put PGPASSWORD in environment for psql to pick up
export PGPASSWORD=${POSTGRES_PASSWORD}
# Migrate using pgloader
# NOTE The following 2 approaches work:
#   pgloader $PRG_PATH/migrate_sqlite_psql.load
#   pgloader sqlite:///home/scblack/.tms/database/tms.db pgsql://tms@localhost:5431/tmsdb
# NOTE This approach does not, because the sqlite3 library in the docker image is older
#   docker run --network=host -e PGPASSWORD="${POSTGRES_PASSWORD}" --rm --name pgloader \
#     -v "$PRG_PATH":/load -v "$TMS_HOME":/data \
#     dimitri/pgloader:v3.6.7 pgloader /load/migrate_sqlite_psql.load
# If necessary, in the future we can always build our own docker image using latest pgloader source and sqlite3 library.
# For now, assume pgloader is installed when we run.

# Create a pgloader load file at a temporary location
TMP_FILE=$(mktemp)
cat >> $TMP_FILE << EOB
load database
     from sqlite://${TMS_HOME}/database/tms.db
     into pgsql://${TMS_DB_USER}@${TMS_DB_HOST}:${TMS_DB_PORT}/tmsdb
 with include drop, create tables, create indexes, reset sequences
  set work_mem to '16MB', maintenance_work_mem to '512 MB';
EOB

# Run pgloader to migrate everything, including schema and data
pgloader $TMP_FILE

# Clean up
rm -f $TMP_FILE
cd $RUN_DIR
