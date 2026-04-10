#!/bin/bash
#
# Script to test upgrade from 0.2.0 to 0.3.0 which includes migrating from sqlite to postgres
#  - remove previous local install from ~/.tms
#  - restore a saved copy of 0.2.0 install to ~/.tms
#  - setup env variables for upgrade
#  - run upgrade script which should also do the migration from sqlite to postgres
#  - as final check run tms_server to get the version
#
# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

# By default, upgrade script sets install dir to /opt/tms_server, so override for testing.
export TMS_INSTALL_DIR=/tmp/tms_server

# For local test use to standard tms home dir of ~/.tms
export TMS_HOME=~/.tms

# Remove current install
echo "**********************************************************************"
echo "   Removing previous installation"
echo "**********************************************************************"
rm -fr $TMS_HOME

# Simulate a previous 0.2.0 install by restoring a backed up ~/.tms install directory
#   and creating a version file and a fake executable file under /tmp/tms_server
echo "*********************************************************************************"
echo "   Restoring backed up TMS server 0.2.0 install to standard install dir $TMS_HOME"
echo "*********************************************************************************"
TMS_HOME_BAK_DIR=~/dot_tms_bak
/bin/cp -pr $TMS_HOME_BAK_DIR $TMS_HOME
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "TMS server restore failed"
  echo "Exiting ..."
  exit $RET_CODE
fi
mkdir -p $TMS_INSTALL_DIR
touch $TMS_INSTALL_DIR/tms_server
chmod +x $TMS_INSTALL_DIR/tms_server
# Version is not normally in $TMS_HOME, so move it to where it should be, TMS_INSTALL_DIR
mv $TMS_HOME/tms.version $TMS_INSTALL_DIR

# Set up env variables for running the upgrade script
. $PRG_PATH/local.env

echo "*********************************************************************************"
echo "   Running upgrade script"
echo "*********************************************************************************"
# Run the upgrade script in test mode
$PRG_PATH/../deployment/native/upgrade_020_030.sh --test
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "TMS upgrade failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

# Check that upgrade appears to have worked by running tms_server --version
$TMS_INSTALL_DIR/tms_server --version
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "TMS server --version failed"
  echo "Exiting ..."
  exit $RET_CODE
fi

cd $RUN_DIR
echo "*****************"
echo "     SUCCESS"
echo "*****************"