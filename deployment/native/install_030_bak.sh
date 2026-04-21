#!/bin/bash
# Clean install of TMS Server
# Build and deploy the latest native version TMS Server
# This script must be run as root or run in --test mode.
# This script builds a release version and installs it as a service.
#
# Default install directory for server executable and service configuration is /opt/tms_server.
#   May be overridden using env variable TMS_INSTALL_DIR.
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
#  - Following are installed: rust tool chain (cargo, rustc), postgres psql.
#
# Configuration:
#  - Following env variables are set at minimum: POSTGRES_PASSWORD, TMS_DB_USER_PASSWORD
#  - Other env variables that can be set to override defaults:
#    - TMS_DB_HOST    default = localhost
#    - TMS_DB_PORT    default = 5432
#
# A --test mode is supported allowing for execution as a non-root user and tms_install_user is taken to be current user.

# TODO COMMON code between install and upgrade.
PrgName=$(basename "$0")
USAGE="Usage: $PrgName [ <tms_install_user> | --test ]"

# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

# This script is located under deployment/native. Some operations are relative to the top level source directory.
SRC_DIR=$PRG_PATH/../..

# Check number of arguments
if [ $# -gt 1 ]; then
  echo "$USAGE"
  exit 1
fi

# Determine if this is a normal run or a test run
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

# Check that all required env variables are set
FAILED=false
env_list="POSTGRES_PASSWORD TMS_DB_USER_PASSWORD"
for name in $env_list
do
  if [[ -z "${!name}" ]]; then
    echo "Please set env var ${name} before running this script"
    FAILED=true
  fi
done
if [ "$FAILED" = true ]; then
  echo "Please set required environment variables"
  echo "Exiting ..."
  exit 1
fi

# Determine home directory of install user.
if [ "$TEST_MODE" == "true" ]; then
  TMS_HOME="$HOME"
else
  TMS_HOME=$(su - $INSTALL_USR -c 'echo $HOME')
fi

# Define backup script related settings
BAK_DIR="$TMS_HOME/backups"
BAK_FILE="backup_tms_server.sh"
BAK_FILE_PATH="$BAK_DIR/scripts/$BAK_FILE"
# Timestamp to use when backing up existing files
BAK_TIMESTAMP=$(date  +%Y%m%d%H%M%S)

# Make sure rust is installed.
if [ "$TEST_MODE" == "true" ]; then
  rustc --version
  RET_CODE=$?
else
  su - $INSTALL_USR  -c 'rustc --version'
  RET_CODE=$?
fi
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
    echo "ERROR: Unable to access rustc. Install the latest stable version of Rust if necessary."
    echo "Exiting ..."
    exit $RET_CODE
fi

# Make sure the specified user for the TMS install exists
id "$INSTALL_USR" >/dev/null 2>&1
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
    echo "TMS install user does not exist. User: $INSTALL_USR"
    echo "Exiting ..."
    exit $RET_CODE
fi

# Set TMS root directory. Location of config, logs, etc.
ROOT_DEF_DIR="$TMS_HOME/.tms"
if [ -n "$TMS_ROOT_DIR" ]; then
  ROOT_DIR="$TMS_ROOT_DIR"
else
  ROOT_DIR="$ROOT_DEF_DIR"
fi
# TODO COMMON code between install and upgrade.


# TODO/TBD: This is the first install specific step? We really should combine install/upgrade
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
chmod 700 $ROOT_DIR
chown $INSTALL_USR:$INSTALL_USR $ROOT_DIR

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
chown $INSTALL_USR:$INSTALL_USR $LOCAL_DIR

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
chown $INSTALL_USR:$INSTALL_USR $EXEC_DEST

# Update version in install dir
VERS_FILE=$INSTALL_DIR/tms.version
echo "$VERS_NEW" > $VERS_FILE

# First Time Install Processing. Save output to LOCAL_DIR
echo
echo "===== Initialize server. Running tms_server --install as user: $INSTALL_USR"
echo "========================================================================================="
# Initialize the content of the install directory.
INSTALL_INIT_CMD="$EXEC_DEST --install --root-dir $ROOT_DIR"
# We must run from the top of the source code checkout so the files under resources are available
cd $SRC_DIR
$INSTALL_INIT_CMD > ${LOCAL_DIR}/tms-install.out 2>&1
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "ERROR: Error running tms_server init. Command: $INSTALL_INIT_CMD"
  echo "       Please see output here: ${LOCAL_DIR}/tms-install.out"
  echo "Exiting ..."
  exit $RET_CODE
fi
chmod 400 ${LOCAL_DIR}/tms-install.out
chown $INSTALL_USR:$INSTALL_USR $LOCAL_DIR/tms-install.out

# If there are custom tms or log4s config then copy into place
if [ -f $LOCAL_DIR/tms.toml ]; then
  cp -p $LOCAL_DIR/tms.toml $ROOT_DIR/config
  RET_CODE=$?
  if [ $RET_CODE -ne 0 ]; then
    echo "ERROR: Unable to copy: $LOCAL_DIR/tms.toml"
    echo "Exiting ..."
    exit $RET_CODE
  fi
  chmod 600 $ROOT_DIR/config/tms.toml
fi
if [ -f $LOCAL_DIR/log4rs.yml ]; then
  cp -p $LOCAL_DIR/log4rs.yml $ROOT_DIR/config
  RET_CODE=$?
  if [ $RET_CODE -ne 0 ]; then
    echo "ERROR: Unable to copy file: $LOCAL_DIR/log4rs.yml"
    echo "Exiting ..."
    exit $RET_CODE
  fi
  chmod 600 $ROOT_DIR/config/log4rs.toml
fi
chown -R $INSTALL_USR:$INSTALL_USR $ROOT_DIR/config

# If no example cert related files exist then copy example cert and key path files into local directory.
if [ ! -f $LOCAL_DIR/cert.path ]; then
  cp -p $SRC_DIR/deployment/native/cert.path $LOCAL_DIR/cert.path
  chown $INSTALL_USR:$INSTALL_USR $LOCAL_DIR/cert.path
fi
if [ ! -f $LOCAL_DIR/key.path ]; then
  cp -p $SRC_DIR/deployment/native/key.path $LOCAL_DIR/key.path
  chown $INSTALL_USR:$INSTALL_USR $LOCAL_DIR/key.path
fi

# Copy latest backup script into place.
echo
echo "===== Updating backup script. Target path: $BAK_FILE_PATH"
echo "========================================================================================="
mkdir -p $BAK_DIR/scripts
# If backup script currently exists then back it up
if [ -f "$BAK_FILE_PATH" ]; then
  echo "Found existing backup script. Moving it to: ${BAK_FILE_PATH}.bak_${BAK_TIMESTAMP}"
  mv "$BAK_FILE_PATH" "${BAK_FILE_PATH}.bak_${BAK_TIMESTAMP}"
  if [ $RET_CODE -ne 0 ]; then
    echo "ERROR: Unable to move original backup script"
    echo "Exiting ..."
    exit $RET_CODE
  fi
fi
cp -pr "${SRC_DIR}/backup/$BAK_FILE" "$BAK_FILE_PATH"
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo "ERROR: Unable to copy backup script FROM: ${SRC_DIR}/backup/$BAK_FILE TO: $BAK_FILE_PATH"
  echo "Exiting ..."
  exit $RET_CODE
fi
chmod +x "$BAK_FILE_PATH"
chown $INSTALL_USR:$INSTALL_USR "$BAK_FILE_PATH"

# Remove the temporary file
rm -f $TMP_FILE
# Switch back to current working directory of invoking user
cd "$RUN_DIR"
