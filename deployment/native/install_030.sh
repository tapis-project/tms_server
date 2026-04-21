#!/bin/bash
# Combined build/upgrade script for TMS Server
# Build and deploy the latest native version TMS Server
# This script must be run as root or run in --test mode.
#
# This script builds a release version and updates files in the install and root directories as needed.
#
# Default install directory is /opt/tms_server. May be overridden using env variable TMS_INSTALL_DIR.
#
# Default root directory is $HOME/.tms as the install user. May be overridden using env variable TMS_ROOT_DIR.
#
# User used to build and install TMS may be given on the command line. Default user is "tms"
#
# Assumptions:
#  - We are running from a checkout of tms_server github repo.
#  - Following are installed: rust tool chain (cargo, rustc), postgres psql.
#  - If this is an upgrade from TMS version 0.2.0 then SQLite must also be installed.
#
# Configuration:
#  - Following env variables are set at minimum: POSTGRES_PASSWORD, TMS_DB_USER_PASSWORD
#  - Other env variables that can be set to override defaults:
#    - TMS_DB_HOST    default = localhost
#    - TMS_DB_PORT    default = 5432
#
# A --test mode is supported allowing for execution as a non-root user and tms_install_user is taken to be current user.

PrgName=$(basename "$0")

# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

# This script is located under deployment/native. Some operations are relative to the top level source directory.
SRC_DIR=$PRG_PATH/../..

# Define USAGE message
function usage() {
  echo "$PrgName [--upgrade] [--test] [--user install_user]"
  echo "OPTIONS:"
  echo "     --upgrade"
  echo "        This is an upgrade. Default is install."
  echo "     --test"
  echo "        Run in test mode as non-root user. Default is to require running as root user."
  echo "     --user"
  echo "        The TMS install user. Default is tms. In test mode this will always be the current user."
  exit 1
}

# Process command line arguments
UPGRADE=false
TEST_MODE=false
while [[ $# -gt 0 ]]; do
  case $1 in
    --user)
      USR="$2"
      shift # past argument
      shift # past value
      # If no user given or --user followed by one of the other args then abort
      if [ -z "$USR" ] || [ "$USR" == "--upgrade" ] || [ "$USR" == "--test" ]; then
        usage
      fi
      ;;
    --upgrade)
      UPGRADE=true
      shift # past argument
      ;;
    --test)
      TEST_MODE=true
      shift # past argument
      ;;
    -*)
      echo "Unknown option $1"
      usage
      ;;
    *)
      echo "Unknown positional argument $1"
      usage
  esac
done

# Disallow use of --test with --user
if [ "$TEST_MODE" == "true" ] && [ -n "$USR" ]; then
  echo "--test may not be used with --user"
  usage
fi

# Make sure we are running as root or are in test mode
if [ "$TEST_MODE" == "false" ] && [ "$EUID" != 0 ]; then
  echo "This program must be run as the root user or in test mode"
  echo "Exiting ..."
  usage
fi

# Determine TMS install user
if [ "$TEST_MODE" == "true" ]; then
  INSTALL_USR=$USER
elif [ -n "$USR" ]; then
  INSTALL_USR="$USR"
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

# Set installation directory.
INSTALL_DEF_DIR="/opt/tms_server"
if [ -n "$TMS_INSTALL_DIR" ]; then
  INSTALL_DIR="$TMS_INSTALL_DIR"
else
  INSTALL_DIR="$INSTALL_DEF_DIR"
fi

# Determine new version
if [ "$TEST_MODE" == "true" ]; then
  VERS_NEW=$(cd $SRC_DIR; cargo pkgid | cut -d "#" -f2)
  RET_CODE=$?
else
  VERS_NEW=$(su - $INSTALL_USR -c 'cd tms_server; cargo pkgid | cut -d "#" -f2')
  RET_CODE=$?
fi
if [ $RET_CODE -ne 0 ]; then
  echo "Error determining new TMS version"
  echo "Exiting ..."
  exit $RET_CODE
fi

# Output configuration
echo "******* Install / Upgrade Settings ************************"
echo "******* TMS Version: $VERS_NEW ********************************"
echo "   TEST_MODE=$TEST_MODE"
echo "   UPGRADE=$UPGRADE"
echo "   INSTALL_USR=$INSTALL_USR"
echo "   ROOT_DIR=$ROOT_DIR"
echo "   INSTALL_DIR=$INSTALL_DIR"
echo "***********************************************************"

# Set paths to source and destination of tms_server executable
EXEC_FILE_SRC=$SRC_DIR/target/release/tms_server
EXEC_FILE_DST=$INSTALL_DIR/tms_server

exit 0

# =====================================================================================
#  Perform install/upgrade specific checks related to previous install
# =====================================================================================
if [ "$UPGRADE" == "true" ]; then
  # There should be an existing installation.
  if [ ! -d "$ROOT_DIR/config" ]; then
    echo "ERROR: Unable to find TMS Server configuration under directory: $ROOT_DIR. Path not found: $ROOT_DIR/config"
    echo "If you have not set env variable TMS_ROOT_DIR, you may do so to specify a non-default path for TMS root."
    echo "Default path for TMS root: $ROOT_DEF_DIR"
    echo "Exiting ..."
    exit 1
  fi
  # There should be an executable in the install dir
  if [ ! -f "$EXEC_FILE_DST" ]; then
    echo "ERROR: Unable to find TMS Server executable at path: $EXEC_FILE_DST"
    echo "If you have not set env variable TMS_INSTALL_DIR, you may do so to specify a non-default path for the installation."
    echo "Default path for the installation: $INSTALL_DEF_DIR"
    echo "Exiting ..."
    exit 1
  fi
else
  # There should not be an existing installation
  if [ -e "$ROOT_DIR/config" ]; then
    echo "ERROR: TMS Server appears to already be installed under root directory: $ROOT_DIR. Path found: $ROOT_DIR/config"
    echo "Exiting ..."
    exit 1
  fi
  # There should not be an executable in the install dir.
  if [ -e  "$EXEC_FILE_DST" ]; then
    echo "ERROR: Found existing tms_server executable at path: $EXEC_FILE_DST"
    echo "Exiting ..."
    exit 1
  fi
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
echo "VERS_OLD=$VERS_OLD" >> $TMP_FILE
echo "VERS_NEW=$VERS_NEW" >> $TMP_FILE

# Construct second part of script
cat >> $TMP_FILE << EOB
echo "Upgrading TMS Server from version $VERS_OLD to version $VERS_NEW"
echo "Install directory: $INSTALL_DIR"

# Build executable
echo "Building executable from directory: $SRC_DIR"
cd $SRC_DIR
cargo build --release
EOB

# Let the install user run the tmp script
chmod 755 $TMP_FILE

# Remove any existing executable
rm -f $EXEC_FILE_SRC
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
if [ ! -f "$EXEC_FILE_SRC" ]; then
  echo "There appears to have been a problem building a new executable. File not found at path: $EXEC_FILE_SRC"
  echo "Please check for build errors"
  echo "Exiting ..."
  exit 1
fi

# =====================================================================================
#  Begin install/upgrade specific code
# =====================================================================================








# Determine existing version
VERS_FILE=$INSTALL_DIR/tms.version
if [ ! -f "$VERS_FILE" ]; then
  echo "Unable to determine existing TMS version. Cannot find file: $VERS_FILE"
  echo "Exiting ..."
  exit 1
fi
VERS_OLD=$(cat $VERS_FILE)



# Shut down the service, copy the new executable into place
if [ "$TEST_MODE" != "true" ]; then
  echo
  echo "===== Stopping TMS service and copying new executable into place"
  echo "========================================================================================="
  systemctl stop tms_server
fi
cp $EXEC_FILE_SRC $EXEC_FILE_DST
chown $INSTALL_USR:$INSTALL_USR $EXEC_FILE_DST
chmod 770 $EXEC_FILE_DST

# If there is a customizations directory then rename it to local to match layout as of 0.3.0
if [ -d ${TMS_HOME}/tms_customizations ]; then
  echo
  echo "===== Moving customizations directory from ${TMS_HOME}/tms_customizations to $ROOT_DIR/local"
  echo "========================================================================================="
  mv "${TMS_HOME}/tms_customizations" "$ROOT_DIR/local"
fi

# Update migrations files.
mv "$ROOT_DIR/migrations" "${ROOT_DIR}/migrations.bak_${BAK_TIMESTAMP}"
cp -pr "${SRC_DIR}/resources/migrations" "${ROOT_DIR}/migrations"
chmod 0700 "${ROOT_DIR}/migrations"
chown $INSTALL_USR:$INSTALL_USR "${ROOT_DIR}/migrations"

# Before updating version and starting up new tms_server perform the migration from sqlite to postgres
echo
echo "===== Migrating DB from sqlite to postgres"
echo "========================================================================================="
# Fill in some defaults as needed before running migration
TMS_TEST_MODE=$TEST_MODE
TMS_USR=$INSTALL_USR
if [ -z "$TMS_DB_HOST" ]; then TMS_DB_HOST="localhost"; fi
if [ -z "$TMS_DB_PORT" ]; then TMS_DB_PORT="5432"; fi
# Set TMS_ROOT_DIR and TMS_INSTALL_DIR to the final resolved values
TMS_ROOT_DIR=$ROOT_DIR
TMS_INSTALL_DIR=$INSTALL_DIR
TMS_VERS_NEW=$VERS_NEW
export TMS_DB_HOST TMS_DB_PORT TMS_ROOT_DIR TMS_INSTALL_DIR TMS_TEST_MODE TMS_VERS_NEW TMS_USR

$SRC_DIR/migrate_to_psql/migrate_from_sqlite.sh
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo
  echo "*************** Error running migration script"
  echo "Exiting ..."
  exit $RET_CODE
fi

# Copy latest backup script into place.
mkdir -p $BAK_DIR/scripts
echo
echo "===== Updating backup script. Target path: $BAK_FILE_PATH"
echo "========================================================================================="
# If backup script currently exists then back it up
if [ -f "$BAK_FILE_PATH" ]; then
  # First check to see if we need to back it up.
  diff -q "${SRC_DIR}/backup/$BAK_FILE" "$BAK_FILE_PATH" 1>/dev/null
  RET_CODE=$?
  if [ $RET_CODE -ne 0 ]; then
    echo "Found existing backup script. Moving it to: ${BAK_FILE_PATH}.bak_${BAK_TIMESTAMP}"
    mv "$BAK_FILE_PATH" "${BAK_FILE_PATH}.bak_${BAK_TIMESTAMP}"
  fi
fi
cp -p "${SRC_DIR}/backup/$BAK_FILE" "$BAK_FILE_PATH"
chown $INSTALL_USR:$INSTALL_USR "$BAK_FILE_PATH"

# TODO Create environment file for backup script

# Update version in install dir
echo "$VERS_NEW" > $VERS_FILE

# Configure and start the service
if [ "$TEST_MODE" != "true" ]; then
  echo
  echo "===== Configuring TMS service"
  echo "========================================================================================="
    SVC_CFG_DIR="$INSTALL_DIR/lib/systemd/system"
    SVC_CFG_PATH="$SVC_CFG_DIR/tms_server.service"
    SVC_ENV_PATH="$ROOT_DIR/local/tms_service.env"
    mkdir -p $SVC_CFG_DIR
    # TODO Copy service config into place
    cp -p "${SRC_DIR}/deployment/native/tms_server.service" "$SVC_CFG_PATH"



    # TODO Update service file to point to executable and environment settings file
    echo "ExecStart=$EXEC_FILE_DST" >> $SVC_CFG_PATH
    echo "EnvironmentFile=$ROOT_DIR/local/tms_service.env" >> $SVC_CFG_PATH
  echo
  echo "===== Starting TMS service"
  echo "========================================================================================="
  # TODO Create environment file for service

  # TODO Update service config file at /opt/tms_server/lib/systemd/system/tms_server.service to point to env file.

  systemctl start tms_server
fi

# Remove the temporary file
rm -f $TMP_FILE
# Switch back to current working directory of invoking user
cd "$RUN_DIR"
