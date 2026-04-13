#!/bin/bash
# Clean install of TMS Server
# Build and deploy the latest native version TMS Server
# This script must be run as root or run in --test mode.
# This script builds a release version and updates files in the install directory as needed.
#
# Default install directory is /opt/tms_server. May be overridden using env variable TMS_INSTALL_DIR.
#
# Default root directory is ~/.tms. May be overridden using env variable TMS_ROOT_DIR.
#
# User may define TMS_LOCAL_DIR for local customizations. Default is $TMS_ROOT_DIR/local
# Local directory will contain install output and may contain custom tms.toml and log4rs.yml files.
#
# User used to build and install TMS may be given on the command line. Default user is "tms"
#
# Assumptions:
#  - We are running from a checkout of tms_server github repo.
#  - Following are installed: rust tool chain (cargo, rustc).
#  - TMS postgres DB is set up and available.
#  - Following env variables are set:
#    - TMS_DB_HOST     e.g. localhost
#    - TMS_DB_PORT     e.g. 5431
#    - TMS_DB_USER     e.g. tms
#    - TMS_DB_USER_PASSWORD
#    - POSTGRES_PASSWORD
#
# A --test mode is supported allowing for execution as a non-root user and tms_install_user is taken to be current user.

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

# Set root directory. Location of config, logs, etc.
ROOT_DEF_DIR="$HOME/.tms"
if [ -n "$TMS_ROOT_DIR" ]; then
  ROOT_DIR="$TMS_ROOT_DIR"
else
  ROOT_DIR="$ROOT_DEF_DIR"
fi
# This is an install, so there should not be an existing installation
if [ -e "$ROOT_DIR/config" ]; then
  echo "ERROR: TMS Server appears to already be installed under root directory: $ROOT_DIR. Path found: $ROOT_DIR/config"
  echo "Exiting ..."
  exit 1
fi
# Create the root directory
mkdir -p $ROOT_DIR
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "ERROR: Unable to create TMS directory: $ROOT_DIR"
  echo "Exiting ..."
  exit $RET_CODE
fi

# Set installation directory. Location of tms_server executable and tms.version.
INSTALL_DEF_DIR="/opt/tms_server"
if [ -n "$TMS_INSTALL_DIR" ]; then
  INSTALL_DIR="$TMS_INSTALL_DIR"
else
  INSTALL_DIR="$INSTALL_DEF_DIR"
fi
# If necessary create the install directory.
if [ ! -d "$INSTALL_DIR" ]; then
  mkdir -p $INSTALL_DIR
  RET_CODE=$?
  if [ $RET_CODE -ne 0 ]; then
    echo "ERROR: Unable to create TMS install directory: $INSTALL_DIR"
    echo "Exiting ..."
    exit $RET_CODE
  fi
fi

# Set local directory. Location of install output and optional custom tms.toml, log4rs.yml files.
LOCAL_DEF_DIR="$ROOT_DIR/local"
if [ -n "$TMS_LOCAL_DIR" ]; then
  LOCAL_DIR="$TMS_LOCAL_DIR"
else
  LOCAL_DIR="$LOCAL_DEF_DIR"
fi
# Create the local directory. NOTE: It may have already been created and pre-populated with custom files.
mkdir -p $LOCAL_DIR
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "ERROR: Unable to create directory: $LOCAL_DIR"
  echo "Exiting ..."
  exit $RET_CODE
fi
# Restrict it since it will contain secrets from the initialization run.
chmod 700 $LOCAL_DIR

# Determine new version
if [ "$TEST_MODE" == "true" ]; then
  VERS_NEW=$(cd $SRC_DIR; cargo pkgid | cut -d "#" -f2)
else
  VERS_NEW=$(su - $INSTALL_USR -c 'cd tms_server; cargo pkgid | cut -d "#" -f2')
fi

# Check to see if TMS appears to already be installed.
EXEC_DEST="$TMS_INSTALL_DIR/tms_server"
# Make sure TMS executable is not already installed
if [ -e "$EXEC_DEST" ]; then
  echo "ERROR It appears that TMS is already installed under directory: $TMS_INSTALL_DIR. Found: $EXEC_DEST"
  echo "Exiting ..."
  exit 1
fi

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
echo "Installing TMS Server. Version $VERS_NEW"
echo "Install directory: $INSTALL_DIR"
echo "Install user: $INSTALL_USR"

# Build executable
echo "Building executable from directory: $SRC_DIR"
cd $SRC_DIR
cargo build --release
EOB

# Set path to built executable
EXEC_FILE=$SRC_DIR/target/release/tms_server
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

# Copy the new executable into place
cp $EXEC_FILE $EXEC_DEST
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "ERROR: Unable to copy executable from path: $EXEC_FILE to path: $EXEC_DEST"
  echo "Exiting ..."
  exit $RET_CODE
fi
chmod 770 $EXEC_DEST

# Update version in install dir
VERS_FILE=$INSTALL_DIR/tms.version
echo "$VERS_NEW" > $VERS_FILE

# First Time Install Processing. Save output to LOCAL_DIR
echo
echo "===== Initialize server. Running tms_server --install as user: $INSTALL_USR"
echo "========================================================================================="
# Initialize the content of the install directory.
set -xv
INSTALL_INIT_CMD="$EXEC_DEST --install --root-dir $ROOT_DIR"
$INSTALL_INIT_CMD > ${LOCAL_DIR}/tms-install.out 2>&1
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "ERROR: Error running tms_server init. Command: $INSTALL_INIT_CMD"
  echo "Exiting ..."
  exit $RET_CODE
fi
chmod 600 ${LOCAL_DIR}/tms-install.out

# TODO
exit 0

# If there are custom tms or log4s config then copy into place
if [ -f $LOCAL_DIR/tms.toml ]; then
  cp -p $LOCAL_DIR/tms.toml $ROOT_DIR/tms.toml
  RET_CODE=$?
  if [ $RET_CODE -ne 0 ]; then
    echo "ERROR: Unable to copy: $LOCAL_DIR/tms.toml"
    echo "Exiting ..."
    exit $RET_CODE
  fi
  chmod 600 $ROOT_DIR/tms.toml
fi
if [ -f $LOCAL_DIR/log4rs.yml ]; then
  cp -p $LOCAL_DIR/log4rs.yml $ROOT_DIR/log4rs.yml
  RET_CODE=$?
  if [ $RET_CODE -ne 0 ]; then
    echo "ERROR: Unable to copy file: $LOCAL_DIR/log4rs.yml"
    echo "Exiting ..."
    exit $RET_CODE
  fi
  chmod 600 $ROOT_DIR/tms.toml
fi

# If no example cert related files exist then copy example cert and key path files into local directory.
if [ ! -f $LOCAL_DIR/cert.path ]; then
  cp -p $SRC_DIR/deployment/native/cert.path $LOCAL_DIR/cert.path
fi
if [ ! -f $LOCAL_DIR/key.path ]; then
  cp -p $SRC_DIR/deployment/native/key.path $LOCAL_DIR/key.path
fi



#
# TODO
# TODO
# Start the service
if [ "$TEST_MODE" != "true" ]; then
  echo
  echo "===== Starting TMS service"
  echo "========================================================================================="
  systemctl start tms_server
fi

# Remove the temporary file
rm -f $TMP_FILE
# Switch back to current working directory of invoking user
cd "$RUN_DIR"
