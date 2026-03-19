#!/bin/bash
#
# Script to reset a TMS server install for local testing:
#  - rebuild tms
#  - remove current install
#  - reset the DB
#  - re-install
#  - Update config for local testing
#
# Determine absolute path to location from which we are running and change to that directory.
export RUN_DIR=$(pwd)
export PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
export PRG_PATH=$(pwd)

# Rebuild tms
echo "**********************************************************************"
echo "   Rebuilding TMS server"
echo "**********************************************************************"
cd ..
cargo build
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "TMS server build failed"
  echo "Exiting ..."
  exit $RET_CODE
fi

#  Reset the DB
./deployment/tms_drop_db.sh
./deployment/tms_init_db.sh

# Remove current install and re-install
echo "**********************************************************************"
echo "   Removing previous installation and re-installing"
echo "**********************************************************************"
rm -fr ~/.tms
./target/debug/tms_server --install
# Update config for local testing
echo "**********************************************************************"
echo "   Updating configuration for local testing"
echo "**********************************************************************"
/bin/cp -f $PRG_PATH/tms_test_local.toml ~/.tms/config/tms.toml
