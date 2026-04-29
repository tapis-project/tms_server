#!/bin/bash
#
# Script to create backup of TMS server DB and push to s3
# NOTE: Based on tapis-deployer backup scripts located at tapisdeploy:/home/tapisprod/cron/scripts
#
# NOTE: If default installation directories are not used for TMS, please update TMS_ROOT and TMS_HOME
#
# Support backup for two types of DB, postgres and sqlite. For TMS 0.2.0 and earlier use SQLITE.
#    For 0.3.0 and later use POSTGRES.
#
# For POSTGRES the file ~/.tms/local/tms-db-env must contain the DB configuration, for example:
#   PG_USER=tms
#   PG_DBNAME=tmsdb
#   PG_HOST="localhost"
#   PG_PORT="5432"
#   PG_PASSWORD="*******"
#   PG_DEPLOYMENT="tms-postgres"
#
# Crontab entry to run at 4am every day:
#
# 4 0 * * * /home/tms/tms_server/backups/scripts/backup_tms_server.sh
#
# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

# Set DB_TYPE. Two types supported, SQLITE and POSTGRES
DB_TYPE="POSTGRES"

# Set TMS root and home directories to the default. Customize as needed.
TMS_ROOT="$HOME/.tms"
TMS_HOME="$HOME"
SERVICE=tms
HOST=$(hostname)

bucketname=${SERVICE}-${HOST}-backups

backuptimestamp=$(date +%Y%m%d)
backupyear=$(date +%Y)
backupdir="$TMS_HOME/backups/${SERVICE}"

# Set some DB_TYPE specific parameters
if [ "$DB_TYPE" == "SQLITE" ]; then
  backupfilename=${HOST}-${SERVICE}-backup-${backuptimestamp}.sq3
  backupsrcdb="$TMS_ROOT/database/tms.db"
elif [ "$DB_TYPE" == "POSTGRES" ]; then
  backupfilename=${HOST}-${SERVICE}-backup-${backuptimestamp}.sql
  source "$TMS_ROOT/local/tms-db-env"
  PG_URL="postgresql://$PG_USER:$PG_PASSWORD@$PG_HOST:$PG_PORT/$PG_DBNAME"
else
  echo "Unsupported DB_TYPE: $DB_TYPE"
  echo "Exiting ..."
  exit 1
fi

backupfilepath="${backupdir}/${backupfilename}"
backupfilegz="${backupfilepath}.gz"
backups3path="s3://${bucketname}/${backupyear}/"

mkdir -p "${backupdir}"

# If file exists then log error and exit
if [ -f "${backupfilegz}" ]; then
  echo "ERROR: ${SERVICE}-${HOST} backup failed"
  echo "${SERVICE}-${HOST} backup failed. File already exists. File: ${backupfilegz}"
  echo "Exiting ..."
  exit 1
fi

# Make the backup based on DB_TYPE
if [ "$DB_TYPE" == "SQLITE" ]; then
  echo "Creating backup using sqlite3 .backup command"
  sqlite3 "${backupsrcdb}" ".backup ${backupfilepath}"
  RET_CODE=$?
elif [ "$DB_TYPE" == "POSTGRES" ]; then
  echo "Creating backup using postgres pg_dump command"
  docker exec -it "$PG_DEPLOYMENT" /bin/bash -c "pg_dump --dbname=$PG_URL" > "${backupfilepath}"
  RET_CODE=$?
else
  echo "Unsupported DB_TYPE: $DB_TYPE"
  echo "Exiting ..."
  exit 1
fi
# Check return code of backup execution
if [ $RET_CODE -ne 0 ]; then
  echo "Backup command failed for DB_TYPE=$DB_TYPE"
  echo "Exiting ..."
  exit $RET_CODE
fi

# Compress the backup
echo "Compressing the backup file"
gzip "${backupfilepath}"
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "TMS backup compress failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

# Make sure we got a gzip file
if [ ! -f "${backupfilegz}" ]; then
  echo "ERROR: Backup failed. Unable to find compressed file: ${backupfilegz} "
  echo "Exiting ..."
  exit 1
fi

# Make the bucket in case this is the first run
s3cmd mb "s3://${bucketname}"

# Push the compressed backup to s3
echo "Pushing backup file to s3 using path: ${backups3path}"
s3cmd put "${backupfilegz}" "${backups3path}"
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "TMS backup s3 push failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

# We are done
echo "${SERVICE}-${HOST} backup success"
echo "Listing backups at ${backups3path}"
s3cmd ls "${backups3path}"

cd $RUN_DIR
