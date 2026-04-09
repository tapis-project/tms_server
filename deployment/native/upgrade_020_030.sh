#!/bin/bash
# Build and deploy the latest native version TMS Server
# This script must be run as root.
# This script builds a release version and updates files in the install directory as needed.
# Default install directory is /opt/tms_server. May be overridden using env variable TMS_INSTALL_DIR.
# User used to build and install TMS may be given on the command line. Default user is "tms"
#
# Assumptions:
#  - We are running from a checkout of tms_server github repo.
#  - Following are installed: rust tool chain (cargo, rustc), SQLite and postgres psql.
#  - Following env variables are set:
#    - TMS_DB_HOST     e.g. localhost
#    - TMS_DB_PORT     e.g. 5431
#    - TMS_DB_USER     e.g. tms
#    - TMS_DB_PASSWORD
#    - POSTGRES_PASSWORD
#
# A test mode is supported allowing for execution as a non-root user and tms_install_user is taken to be current user.

PrgName=$(basename "$0")
USAGE="Usage: $PrgName [ <tms_install_user> | --test ]"

# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

# Upgrade script is located under deployment/native. Some operations are relative to the top level source directory.
SRC_DIR=$PRG_PATH/../..

# Check number of arguments
if [ $# -gt 1 ]; then
  echo "$USAGE"
  exit 1
fi

TEST_MODE=false
if [ "$1" == "--test" ]; then
  echo "*******************************"
  echo "     Running in test mode"
  echo "*******************************"
  TEST_MODE=true
fi

# Make sure we are running as root or are in test mode
if [ "$TEST_MODE" == "false" ] && [ "$EUID" != 0 ]; then
  echo "This program must be run as the root user"
  echo "Exiting ..."
  exit 1
fi

# Determine TMS install user
if [ "$TEST_MODE" == "true" ]; then
  INSTALL_USR=$USER
elif [ -n "$1" ]; then
  INSTALL_USR="$1"
else
  INSTALL_USR=tms
fi

# Make sure the specified user for the TMS install exists
id "$INSTALL_USR" >/dev/null 2>&1
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
    echo "TMS install user does not exist. User: $INSTALL_USR"
    echo "Exiting ..."
    exit $RET_CODE
fi

# Set installation directory.
INSTALL_DEF_DIR="/opt/tms_server"
if [ -n "$TMS_INSTALL_DIR" ]; then
  INSTALL_DIR="$TMS_INSTALL_DIR"
else
  INSTALL_DIR="$INSTALL_DEF_DIR"
fi

# Make sure we have the expected executable in the install dir
if [ ! -f "$INSTALL_DIR/tms_server" ]; then
  echo "Unable to find TMS Server executable at path: $INSTALL_DIR"
  echo "If you have not set env variable TMS_INSTALL_DIR you may do so to specify a non-default path for the installation."
  echo "Default path for the installation: $INSTALL_DEF_DIR"
  echo "Exiting ..."
  exit 1
fi

# Determine existing version
VERS_FILE=$INSTALL_DIR/tms.version
if [ ! -f "$VERS_FILE" ]; then
  echo "Unable to determine existing TMS version. Cannot find file: $VERS_FILE"
  echo "Exiting ..."
  exit 1
fi
VERS_OLD=$(cat $VERS_FILE)

# Determine new version
if [ "$TEST_MODE" == "true" ]; then
  VERS_NEW=$(cd $SRC_DIR; cargo pkgid | cut -d "#" -f2)
else
  VERS_NEW=$(su - $INSTALL_USR -c 'cd tms_server; cargo pkgid | cut -d "#" -f2')
fi

# Set path to built executable
EXEC_FILE=$SRC_DIR/target/release/tms_server

# Construct script to be used by install user to build new executable
echo
echo "===== Creating script for building new executable"
echo "========================================================================================="
TMP_FILE=$(mktemp)
# Construct first part of script
echo "#!/bin/bash" > $TMP_FILE
# Place various env variables into script
echo "SRC_DIR=$SRC_DIR" >> $TMP_FILE
echo "INSTALL_DIR=$INSTALL_DIR" >> $TMP_FILE
echo "VER_OLD=$VERS_OLD" >> $TMP_FILE
echo "VER_NEW=$VERS_NEW" >> $TMP_FILE

# Construct second part of script
cat >> $TMP_FILE << EOB
echo "Upgrading TMS Server from version $VERS_OLD to version $VERS_NEW"
echo "Install directory: $INSTALL_DIR"

# Build executable
echo "Building executable from directory: $SRC_DIR"
cd $SRC_DIR
cargo build --release
EOB

# Remove any existing executable
rm -f $EXEC_FILE
# Run the script to build the new executable
echo
echo "===== Running build script as TMS install user. User: $INSTALL_USR"
echo "========================================================================================="
chmod +x $TMP_FILE
chown $INSTALL_USR:$INSTALL_USR $TMP_FILE
if [ "$TEST_MODE" != "true" ]; then
  su - $INSTALL_USR -c "$TMP_FILE"
  RET_CODE=$?
else
  $TMP_FILE
  RET_CODE=$?
fi
echo "========================================================================================="
if [ $RET_CODE -ne 0 ]; then
  echo
  echo "*************** Error running build script"
  echo "Exiting ..."
  exit $RET_CODE
fi

# Make sure executable was built
if [ ! -f "$EXEC_FILE" ]; then
  echo "There appears to have been a problem building a new executable. File not found at path: $EXEC_FILE"
  echo "Please check for build errors"
  echo "Exiting ..."
  exit 1
fi

# Shut down the service, copy the new executable into place
if [ "$TEST_MODE" != "true" ]; then
  echo
  echo "===== Stopping TMS service and copying new executable into place"
  echo "========================================================================================="
  systemctl stop tms_server
fi
cp $EXEC_FILE $INSTALL_DIR/tms_server

# Update migrations files.
BAK_TIMESTAMP=`date  +%Y%m%d%H%M%S`
mv $TMS_HOME/migrations $TMS_HOME/migrations_bak_$BAK_TIMESTAMP
cp -pr $SRC_DIR/resources/migrations $TMS_HOME/migrations
chmod 0700 $TMS_HOME/migrations

# Before updating version and starting up new tms_server perform the migration from sqlite to postgres
echo
echo "===== Migrating DB from sqlite to postgres"
echo "========================================================================================="

$SRC_DIR/migrate_to_psql/migrate_from_sqlite.sh
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo
  echo "*************** Error running migration script"
  echo "Exiting ..."
  exit $RET_CODE
fi

# Update version in install dir
echo "$VERS_NEW" > $VERS_FILE

# Start the service
echo
echo "===== Starting TMS service"
echo "========================================================================================="
if [ "$TEST_MODE" != "true" ]; then
  systemctl start tms_server
fi

# TODO Remove the temporary file
# rm -f $TMP_FILE
# Switch back to current working directory of invoking user
cd "$RUN_DIR"
