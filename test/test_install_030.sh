#!/bin/bash
#
# Script to test clean install of 0.3.0
#  - remove previous local install from ~/.tms
#  - setup env variables for install
#  - run tms_server --install for initialization
#  - copy tms_server executable into place
#  - as final check run tms_server to get the version
#
# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

# Some operations are relative to the top level source directory.
SRC_DIR=$PRG_PATH/..

# Directory for location of executable and version. Override for local test. Default is under /opt.
export TMS_INSTALL_DIR=/tmp/tms_server

# Remove current install
echo "**********************************************************************"
echo "   Removing previous installation"
echo "**********************************************************************"
# For local test use standard tms home dir of ~/.tms
export TMS_ROOT_DIR=~/.tms
rm -fr $TMS_ROOT_DIR
rm -f $TMS_INSTALL_DIR/tms_server
rm -f $TMS_INSTALL_DIR/tms.version

# Set up env variables for running install
. $PRG_PATH/local_install.env

# Reset the postgres DB
echo "**********************************************************************"
echo "   Initializing Postgres DB for TMS"
echo "**********************************************************************"
$SRC_DIR/migrate_to_psql/tms_drop_db.sh
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "tms_drop_db failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

$SRC_DIR/migrate_to_psql/tms_init_db.sh
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "tms_init_db failed."
  echo "Exiting ..."
  exit $RET_CODE
fi

# Set up TMS local dir to have a custom tms.toml for local testing
# TMS_LOCAL_DIR used for install output and custom tms.toml, log4rs.yml files.
export TMS_LOCAL_DIR=$TMS_ROOT_DIR/local
mkdir -p $LOCAL_DIR
chmod 700 $LOCAL_DIR
cp -p $SRC_DIR/test/tms_test_local.toml $LOCAL_DIR/tms.toml
chmod 600 $LOCAL_DIR/tms.toml

# Run the install
cd $SRC_DIR
$SRC_DIR/deployment/native/install_030.sh --test
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "Install of tms_server failed."
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
