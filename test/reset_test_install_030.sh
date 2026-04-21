#!/bin/bash
#
# Script to reset a TMS server install for local testing:
#  - rebuild tms
#  - remove current install
#  - reset the DB
#
# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

# Some operations are relative to the top level source directory.
SRC_DIR=$PRG_PATH/..

# Check that all required env variables are set
FAILED=false
env_list="POSTGRES_PASSWORD TMS_DB_USER_PASSWORD"
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

# Move to the top level of tms_server src code directory
cd $SRC_DIR || exit 1

# Rebuild tms
echo "**********************************************************************"
echo "   Rebuilding TMS server"
echo "**********************************************************************"
cargo build --release
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "TMS server build failed"
  echo "Exiting ..."
  exit $RET_CODE
fi

#  Reset the DB
$SRC_DIR/deployment/tms_drop_db.sh
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "tms_drop_db failed."
  echo "Exiting ..."
  exit $RET_CODE
fi
$SRC_DIR/deployment/tms_init_db.sh
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "tms_init_db failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

# Remove current install
echo "**********************************************************************"
echo "   Removing current installation"
echo "**********************************************************************"
rm -fr ~/.tms

echo "**********************************************************************"
echo "   Removing service exec and config"
echo "**********************************************************************"
rm /opt/tms_server/tms*
rm -fr /opt/tms_server/lib

cd $RUN_DIR