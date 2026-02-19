#!/bin/bash
# Build and deploy the latest native version TMS Server
# This script must be run as root.
# This script builds a release version and updates files in the install directory as needed.
# Default install directory is /opt/tms_server. May be overridden using env variable TMS_INSTALL_DIR.
# User used to build and install TMS may be given on the command line. Default user is "tms"

PrgName=$(basename "$0")

USAGE="Usage: $PrgName [ <tms_install_user> ]"

# Check number of arguments
if [ $# -gt 1 ]; then
  echo "$USAGE"
  exit 1
fi

# Make sure we are running as root
if [ "$EUID" != 0 ]; then
  echo "This program must be run as the root user"
  echo "Exiting ..."
  exit 1
fi

# Determine TMS install user
INSTALL_USR=tms
if [ -n "$1" ]; then
  INSTALL_USR="$1"
fi

# Make sure the specified user for the TMS install exists
id "$INSTALL_USR" #>/dev/null 2>&1
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
    echo "TMS install user does not exist. User: $INSTALL_USR"
    echo "Exiting ..."
    exit $RET_CODE
fi

# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

# Set installation directory
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
VERS_NEW=$(su - $INSTALL_USR -c 'cd tms_server; cargo pkgid | cut -d "#" -f2')

# Set path to built executable
EXEC_FILE=$PRG_PATH/target/release/tms_server

# Construct script to be used by install user to build new executable
echo
echo "===== Creating script for building new executable"
echo "========================================================================================="
TMP_FILE=$(mktemp)
# Construct first part of script
echo "#!/bin/bash" > $TMP_FILE
# Place various env variables into script
echo "PRG_PATH=$PRG_PATH" >> $TMP_FILE
echo "INSTALL_DIR=$INSTALL_DIR" >> $TMP_FILE
echo "VER_OLD=$VERS_OLD" >> $TMP_FILE
echo "VER_NEW=$VERS_NEW" >> $TMP_FILE

# Construct second part of script
cat >> $TMP_FILE << EOB
echo "Upgrading TMS Server from version $VERS_OLD to version $VERS_NEW"
echo "Install directory: $INSTALL_DIR"

# Build executable
echo "Building executable from directory: $PRG_PATH"
cd $PRG_PATH
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
su - $INSTALL_USR -c "$TMP_FILE"
RET_CODE=$?
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

# Shut down the service, copy the new executable into place, start the service
echo
echo "===== Stopping TMS service and copying new executable into place"
echo "========================================================================================="
systemctl stop tms_server
cp $EXEC_FILE $INSTALL_DIR/tms_server

# Update version in install dir
echo "$VERS_NEW" > $VERS_FILE

echo
echo "===== Starting TMS service"
echo "========================================================================================="
systemctl start tms_server

# Remove the temporary file
rm -f $TMP_FILE
# Switch back to current working directory of invoking user
cd "$RUN_DIR"
