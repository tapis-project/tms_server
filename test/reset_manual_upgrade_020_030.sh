#!/bin/bash
#
# Script to reset a manual test of upgrade script upgrade_020_030.sh
#  - remove previous local install from ~/.tms
#  - restore a saved copy of 0.2.0 install to ~/.tms. Copy most be located at ~/dot_tms_bak
#  - reset the postgres DB
#
# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)
# Some operations are relative to the top level source directory.
SRC_DIR=$PRG_PATH/..

# By default, upgrade script sets install dir to /opt/tms_server, so override for testing.
export TMS_INSTALL_DIR=/opt/tms_server

# Remove current install
echo "**********************************************************************"
echo "   Removing previous installation"
echo "**********************************************************************"
# For local test use to standard tms root dir of ~/.tms
export TMS_ROOT_DIR=~/.tms
rm -fr $TMS_ROOT_DIR
rm -f $TMS_INSTALL_DIR/tms_server
rm -f $TMS_INSTALL_DIR/tms.version

# Simulate a previous 0.2.0 install by restoring a backed up ~/.tms install directory
#   and creating a version file and a fake executable file under /tmp/tms_server
echo "*********************************************************************************"
echo "   Restoring backed up TMS server 0.2.0 install to standard install dir: $TMS_ROOT_DIR"
echo "*********************************************************************************"
ROOT_BAK_DIR=~/dot_tms_bak
/bin/cp -pr $ROOT_BAK_DIR $TMS_ROOT_DIR
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "TMS server restore failed"
  echo "Exiting ..."
  exit $RET_CODE
fi
mkdir -p $TMS_INSTALL_DIR
touch $TMS_INSTALL_DIR/tms_server
chmod +x $TMS_INSTALL_DIR/tms_server
# Version is not normally in $TMS_ROOT_DIR, so move it to where it should be, TMS_INSTALL_DIR
mv $TMS_ROOT_DIR/tms.version $TMS_INSTALL_DIR

# Reset the postgres DB
echo "*********************************************************************************"
echo "   Resetting postgres DB"
echo "*********************************************************************************"
source $TMS_ROOT_DIR/db.env
echo "**********************************************************************"
echo "   Initializing Postgres DB for TMS"
echo "**********************************************************************"
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


cd $RUN_DIR
echo "*****************"
echo "     SUCCESS"
echo "*****************"