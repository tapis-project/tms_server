#!/bin/bash
#
# Script to reset for testing of upgrade including migrating from sqlite to postgres
#  - rebuild tms
#  - reset the postgres DB
#  - remove current install
#  - copy saved old 0.2.0 install to ~/.tms
#
# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

if [ -z "${POSTGRES_PASSWORD}" ]; then
  echo "Please set env var POSTGRES_PASSWORD before running this script"
  exit 1
fi

# Move to the top level of tms_server src code directory
cd $PRG_PATH/.. || exit 1

# Remove current install
echo "**********************************************************************"
echo "   Removing previous installation"
echo "**********************************************************************"
rm -fr ~/.tms

echo "*********************************************************************************"
echo "   Restoring backed up TMS server 0.2.0 install to standard install dir ~/.tms"
echo "*********************************************************************************"
TMS_PREV_DIR=~/dot_tms_bak
/bin/cp -pr $TMS_PREV_DIR ~/.tms
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "TMS server restore failed"
  echo "Exiting ..."
  exit $RET_CODE
fi

#  Reset the postgres DB
echo "**********************************************************************"
echo "   Resetting postgres DB"
echo "**********************************************************************"
./deployment/tms_drop_db.sh
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "tms_drop_db failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

./deployment/tms_init_db.sh
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "tms_init_db failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

# TODO
# TODO Set up env variables for running the upgrade script
# TODO
export TMS_DB_PORT=5431
# By default, upgrade script sets install dir to /opt/tms_server, so override for testing.
export TMS_INSTALL_DIR=/tmp/tms_server
# Create a couple of files to simulate a tms server install
mkdir -p $TMS_INSTALL_DIR
touch $TMS_INSTALL_DIR/tms_server
chmod +x $TMS_INSTALL_DIR/tms_server
cp $TMS_PREV_DIR/tms.version $TMS_INSTALL_DIR

# TODO
# TODO Run the upgrade script in test mode
# TODO
./deployment/native/upgrade.sh --test
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "TMS upgrade failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

# TODO
# TODO Check that upgrade appears to have worked
# TODO

# TODO ???????????????????
# Set up DB migrations directory.
mv ~/.tms/migrations ~/.tms/migrations_bak
cp -pr ./resources/migrations ~/.tms/
chmod 0700 ~/.tms/migrations

echo "*********************************************************************************"
echo "   Creating postgres DB schema"
echo "*********************************************************************************"
export TMS_DB_URL="postgres://tms:password@localhost:5431/tmsdb"
./target/debug/tms_server --schema-only
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "TMS server schema create failed"
  echo "Exiting ..."
  exit $RET_CODE
fi

echo "*****************"
echo "     SUCCESS"
echo "*****************"
cd $RUN_DIR
