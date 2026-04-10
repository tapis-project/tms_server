#!/bin/bash
#
# Script to test clean install of 0.3.0
#  - remove previous local install from ~/.tms
#  - setup env variables for install
#  - run tms_server --install for initialization
#  - copy tms_server executable into place
#  - as final check run tms_server to get the version
#
# TODO
# TODO Convert this to an official native install script.
# TODO
# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

# Some operations are relative to the top level source directory.
SRC_DIR=$PRG_PATH/..

# Directory for location of executable and version
TMS_INSTALL_DIR=/tmp/tms_server

# For local test use to standard tms home dir of ~/.tms
TMS_HOME=~/.tms

# Remove current install
echo "**********************************************************************"
echo "   Removing previous installation"
echo "**********************************************************************"
rm -fr $TMS_HOME

# Set up env variables for running the upgrade script
. $PRG_PATH/local.env

# Reset the postgres DB
echo "**********************************************************************"
echo "   Initializing Postgres DB for TMS"
echo "**********************************************************************"
$PRG_PATH/../migrate_to_psql/tms_drop_db.sh
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "tms_drop_db failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

$PRG_PATH/../migrate_to_psql/tms_init_db.sh
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "tms_init_db failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

# Build executable
cd $SRC_DIR
cargo build
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "Build of tms_server executable failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

echo "*********************************************************************************"
echo "   Running tms_server --install"
echo "*********************************************************************************"
$SRC_DIR/target/debug/tms_server --install
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "tms_server --install failed"
  echo "Exiting ..."
  exit $RET_CODE
fi

# Check that install appears to have worked by running tms_server --version
$TMS_INSTALL_DIR/tms_server --version
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "tms_server --version failed"
  echo "Exiting ..."
  exit $RET_CODE
fi

echo "*****************"
echo "     SUCCESS"
echo "*****************"
cd $RUN_DIR
