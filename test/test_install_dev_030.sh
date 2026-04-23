#!/bin/bash
#
# Script to test clean install of 0.3.0 in the DEV environment, tms-server-dev.tacc.utexas.edu
#  - remove previous install from ~tms/.tms
#  - setup env variables for install
#  - reset the DB
#  - run the install
#  - as final check run tms_server to get the version
#
# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

# Some operations are relative to the top level source directory.
SRC_DIR=$PRG_PATH/..

# Make sure we are running as root
if [ "$EUID" != 0 ]; then
  echo "This program must be run as the root user"
  echo "Exiting ..."
  exit 1
fi

# Remove current install
echo "**********************************************************************"
echo "   Removing previous installation"
echo "**********************************************************************"
# For DEV test use standard tms home dir of ~tms/.tms
# Use hard-coded paths to avoid mistakes
rm -fr ~tms/.tms
rm -fr /opt/tms_server/lib
rm /opt/tms_server/tms_server
rm /opt/tms_server/tms.version
export TMS_INSTALL_DIR=/opt/tms_server
export TMS_ROOT_DIR=~tms/.tms

# Set up env variables for running install
. $PRG_PATH/test_install_dev.env

# Reset the postgres DB
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

if [ -n "$TMS_LOCAL_DIR" ]; then
  LOCAL_DIR="$TMS_LOCAL_DIR"
else
  LOCAL_DIR="$ROOT_DIR/local"
fi
# Set up TMS local dir to have a custom files for testing
# TMS_LOCAL_DIR used for install output and custom tms.toml, log4rs.yml files.
mkdir -p $LOCAL_DIR
chmod 700 $LOCAL_DIR
cp -p $SRC_DIR/test/tms_test_dev.toml $LOCAL_DIR/tms.toml
chmod 600 $LOCAL_DIR/tms.toml

# Run the install
$SRC_DIR/deployment/native/install_030.sh
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
