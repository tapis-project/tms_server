#!/bin/bash
#
# Script to create backup of TMS server DB and push to s3
# NOTE: Based on tapis-deployer backup scripts located on tapisdeploy:/home/tapisprod/cron/scripts
#
# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

TMS_HOME="/home/tms"
SERVICE=tms
ENV=prod

bucketname=${SERVICE}-${ENV}-backups

backuptimestamp=`date +%Y%m%d`
backupyear=$(date +%Y)
backupdir="$TMS_HOME/backups/${SERVICE}"
backupfilename=${ENV}-${SERVICE}-backup-${backuptimestamp}.sq3
backupfilepath="${backupdir}/${backupfilename}"
backupfilegz="${backupfilepath}.gz"
backups3path="s3://${bucketname}/${backupyear}/"

backupsrcdb="$TMS_HOME/.tms/database/tms.db"

mkdir -p ${backupdir}

SQ3_CMD="sqlite3 ${backupsrcdb}"

# If file exists then log error and exit
if [ -f ${backupfilegz} ]; then
  echo "ERROR: ${SERVICE}-${ENV} backup failed"
  echo "${SERVICE}-${ENV} backup failed. File already exists. File: ${backupfilegz} "
  echo "Exiting ..."
  exit 1
fi

# Make the backup
echo "Creating backup using sqlite3 .backup command"
$SQ3_CMD ".backup ${backupfilepath}"
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "sqlite backup command failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

# Compress the backup
echo "Compressing the backup file"
gzip ${backupfilepath}
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "TMS backup compress failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

# Make sure we got a gzip file
if [ ! -f ${backupfilegz} ]; then
  echo "ERROR: Backup failed. Unable to find compressed file: ${backupfilegz} "
  echo "Exiting ..."
  exit 1
fi

# Push the compressed backup to s3
echo "Pushing backup file to s3 using path: ${backups3path}"
s3cmd put ${backupfilegz} ${backups3path}
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "TMS backup s3 push failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

# We are done
echo "${SERVICE}-${ENV} backup success"
echo "Listing backups at ${backups3path}"
s3cmd ls ${backups3path}

cd $RUN_DIR
