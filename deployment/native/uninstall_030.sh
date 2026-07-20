#!/bin/bash
# Uninstall script for TMS Server
#  - Reset the DB leaving only the tms user.
#  - remove all installed files except the local directory
#
# WARNING: This is destructive
# WARNING: Make sure to stop the TMS Server first
# NOTE: This script must be run as the user who installed TMS and owns the files,
#       typically this is user "tms".
#
# Default install directory is /opt/tms_server. May be overridden using env variable TMS_INSTALL_DIR.
#   In test mode this defaults to /tmp/tms_server so any user can perform the service related config.
#
# Default root directory is $HOME/.tms as the install user. May be overridden using env variable TMS_ROOT_DIR.
#
# User may define TMS_LOCAL_DIR for local customizations. Default is $TMS_ROOT_DIR/local
# Local directory will contain install output and may contain custom tms.toml and log4rs.yml files.
#
# User used to build and install TMS may be given on the command line. Default user is "tms"
#
# Assumptions:
#  - We are running from a checkout of tms_server github repo.
#
# Configuration:
#  - Following env variables are set at minimum:
#    - POSTGRES_PASSWORD
#    - TMS_DB_USER_PASSWORD
#  - Other env variables that can be set to override defaults:
#    - TMS_DB_HOST    default = localhost
#    - TMS_DB_PORT    default = 5432
#  - Other less common env variable overrides:
#    - TMS_ROOT_DIR     default = $HOME/.tms
#    - TMS_INSTALL_DIR  default = /opt/tms_server or /tmp/tms_server in test mode
#

PrgName=$(basename "$0")

# An unset variable is an error (avoids silently continuing after a typo in a name)
set -o nounset
# If any of the components of a pipe fails, then the pipe fails
set -o pipefail

# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

# This script is located under deployment/native. Some operations are relative to the top level source directory.
SRC_DIR=$PRG_PATH/../..

# Define USAGE message
function usage() {
  echo "$PrgName --force  [--user install_user]"
  echo "OPTIONS:"
  echo "     --force"
  echo "        Since this is a destructive uninstall this option must be specified"
  echo "     --user"
  echo "        The TMS install user. Default is tms. This must be the current user."
  exit 1
}

# Process command line arguments
UNINSTALL_FORCE=false
while [[ $# -gt 0 ]]; do
  case $1 in
    --user)
      USR="$2"
      shift # past argument
      shift # past value
      # If no user given or --user followed by one of the other args then abort
      if [ -z "$USR" ] || [ "$USR" == "--force" ]; then
        usage
      fi
      ;;
    --force)
      UNINSTALL_FORCE=true
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

# Make sure we are not running as root
if [ "$EUID" == 0 ]; then
  echo "This program must NOT be run as the root user"
  echo "Exiting ..."
  usage
fi

# Check that --force has been specified
if [ "$UNINSTALL_FORCE" == "false" ]; then
  echo "ERROR: This is a destructive operation. You must specify --force"
  usage
fi

# Determine TMS install user
INSTALL_USR="${USR:-tms}"

# Make sure we are running as current user
if [ "$INSTALL_USR" != "$USER" ]; then
  echo "ERROR: This script must be run as the current user. Running as: ${USER}. Expected user: $INSTALL_USR"
  usage
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
TMS_HOME="$HOME"

# Make sure rust and psql are installed.
rustc --version
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
    echo "ERROR: Unable to access rustc. Install the latest stable version of Rust."
    echo "Exiting ..."
    exit $RET_CODE
fi
psql --version
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
    echo "ERROR: Unable to access psql. Install the latest stable version of postgresql-client."
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

# Set installation directory. Location of tms_server, tms.version and lib/
if [ -e "/tmp/tms_server" ]; then
  INSTALL_DEF_DIR="/tmp/tms_server"
else
  INSTALL_DEF_DIR="/opt/tms_server"
fi
if [ -n "$TMS_INSTALL_DIR" ]; then
  INSTALL_DIR="$TMS_INSTALL_DIR"
else
  INSTALL_DIR="$INSTALL_DEF_DIR"
fi

# Set local directory. Location of install output and optional custom tms.toml, log4rs.yml files.
LOCAL_DEF_DIR="$ROOT_DIR/local"
if [ -n "$TMS_LOCAL_DIR" ]; then
  LOCAL_DIR="$TMS_LOCAL_DIR"
else
  LOCAL_DIR="$LOCAL_DEF_DIR"
fi

# Set paths to source and destination of tms_server executable
EXEC_FILE_SRC=$SRC_DIR/target/release/tms_server
EXEC_FILE_DST=$INSTALL_DIR/tms_server

# Set directory for service config file
SVC_CFG_DIR="$INSTALL_DIR/lib/systemd/system"

# Fill in some defaults needed by DB reset
TMS_DB_HOST="${TMS_DB_HOST:-localhost}"
TMS_DB_PORT="${TMS_DB_PORT:-5432}"

# Set location of version file for installed version
VERS_FILE=$INSTALL_DIR/tms.version

# Define backup script related settings
BAK_DIR="$TMS_HOME/backups"
BAK_FILE="backup_tms_server.sh"
BAK_FILE_PATH="$BAK_DIR/scripts/$BAK_FILE"
# Timestamp to use when backing up existing files
BAK_TIMESTAMP=$(date  +%Y%m%d%H%M%S)

# Output configuration
echo "======================================================================================="
echo "======= WARNING ======= WARNING ======= WARNING ======= WARNING ======= WARNING ======="
echo "========================== THIS IS A DESTRUCTIVE OPERATION ============================"
echo "======================================================================================="
echo "******* UnInstall Settings ************************"
echo "******* TMS Version: $VERS_NEW ********************************"
echo "   TEST_MODE=$TEST_MODE"
echo "   INSTALL_USR=$INSTALL_USR"
echo "   ROOT_DIR=$ROOT_DIR"
echo "   LOCAL_DIR=$LOCAL_DIR"
echo "   INSTALL_DIR=$INSTALL_DIR"
echo "   SVC_CFG_DIR=$SVC_CFG_DIR"
echo "   BAK_FILE_PATH=$BAK_FILE_PATH"
echo "   TMS_DB_HOST=$TMS_DB_HOST"
echo "   VERS_FILE=$VERS_FILE"
echo "***********************************************************"
echo
read -p "WARNING DESTRUCTIVE UNINSTALL! Please review above settings. If they are correct enter Y to continue: " resp
case $resp in
  [yY]* ) echo "Continuing ... " ;;
  *) echo "Install cancelled. Exiting ... " ; exit 1 ;;
esac

# =====================================================================================
#  Perform uninstall
# =====================================================================================
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

# ============================================================================================
#  Create directories that might not exist yet.
#  NOTE: We could get fancy and put these in a list, process in a loop and check exit code
# ============================================================================================
mkdir -p $ROOT_DIR
mkdir -p $INSTALL_DIR
mkdir -p $LOCAL_DIR
mkdir -p $SVC_CFG_DIR
mkdir -p $BAK_DIR/scripts

# Restrict some files since they will contain secrets from the initialization run.
chown $INSTALL_USR:$INSTALL_USR $ROOT_DIR
chmod 700 $ROOT_DIR
chown $INSTALL_USR:$INSTALL_USR $LOCAL_DIR
chmod 700 $LOCAL_DIR

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
echo "VERS_NEW=$VERS_NEW" >> $TMP_FILE

# Construct second part of script
cat >> $TMP_FILE << EOB
echo "Building TMS Server version $VERS_NEW"
echo "Install directory: $INSTALL_DIR"

# Build executable
echo "Building executable from directory: $SRC_DIR"
cd $SRC_DIR
cargo build --release > $SRC_DIR/cargo_build.log 2>&1
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
  echo "*************** Please check file cargo_build.log"
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

# Shut down the service
if [ "$TEST_MODE" != "true" ]; then
  echo
  echo "===== Stopping TMS service"
  echo "========================================================================================="
  systemctl stop tms_server
fi

# Copy new tms_server executable into place
cp $EXEC_FILE_SRC $EXEC_FILE_DST
chown $INSTALL_USR:$INSTALL_USR $EXEC_FILE_DST
chmod 770 $EXEC_FILE_DST

# Configure service
echo
echo "===== Configuring TMS service"
echo "========================================================================================="
SVC_CFG_FILE="$SVC_CFG_DIR/tms_server.service"
SVC_ENV_PATH="$LOCAL_DIR/tms_service.env"
# Copy service config into place
cp -p "${SRC_DIR}/deployment/native/tms_server.service" "$SVC_CFG_FILE"
RET_CODE=$?
if [ $RET_CODE -ne 0 ]; then
  echo
  echo "*************** Error copying service config to: $SVC_CFG_FILE"
  echo "Exiting ..."
  exit $RET_CODE
fi

# Update service file to point to executable and environment settings file
echo -e "ExecStart=$EXEC_FILE_DST\n" >> $SVC_CFG_FILE
echo -e "EnvironmentFile=$SVC_ENV_PATH\n" >> $SVC_CFG_FILE

# Create environment file for service
cat >> $SVC_ENV_PATH << EOB
TMS_DB_USER="tms"
TMS_DB_DB_NAME="tmsdb"
TMS_DB_HOST="$TMS_DB_HOST"
TMS_DB_PORT="$TMS_DB_PORT"
TMS_DB_USER_PASSWORD="$TMS_DB_USER_PASSWORD"
EOB
chown $INSTALL_USR:$INSTALL_USR "$SVC_ENV_PATH"
chmod 400 "$SVC_ENV_PATH"

# Copy latest backup script into place.
echo
echo "===== Updating backup script. Target path: $BAK_FILE_PATH"
echo "========================================================================================="
# If backup script currently exists then if necessary back it up
if [ -f "$BAK_FILE_PATH" ]; then
  # First check to see if we need to back it up.
  diff -q "${SRC_DIR}/backup/$BAK_FILE" "$BAK_FILE_PATH" 1>/dev/null
  RET_CODE=$?
  if [ $RET_CODE -ne 0 ]; then
    echo "Found existing backup script. Moving file: ${BAK_FILE_PATH} to file: ${BAK_FILE_PATH}.bak_${BAK_TIMESTAMP}"
    mv "$BAK_FILE_PATH" "${BAK_FILE_PATH}.bak_${BAK_TIMESTAMP}"
  fi
fi
cp -p "${SRC_DIR}/backup/$BAK_FILE" "$BAK_FILE_PATH"
chown $INSTALL_USR:$INSTALL_USR "$BAK_FILE_PATH"
chmod +x "$BAK_FILE_PATH"

# Make sure everything under BAK_DIR is owned by the install user.
chown -R $INSTALL_USR:$INSTALL_USR $BAK_DIR

# Create environment file for backup script
DB_ENV_FILE="$LOCAL_DIR/tms-db-env"
echo
echo "===== Creating environment file for backup script. File path: $DB_ENV_FILE"
echo "========================================================================================="
# Construct env file
cat >> $DB_ENV_FILE << EOB
PG_DEPLOYMENT="tms-postgres"
PG_USER="tms"
PG_DBNAME="tmsdb"
PG_HOST="$TMS_DB_HOST"
PG_PORT="$TMS_DB_PORT"
PG_PASSWORD="$TMS_DB_USER_PASSWORD"
EOB
chown $INSTALL_USR:$INSTALL_USR "$DB_ENV_FILE"
chmod 400 "$DB_ENV_FILE"

# =====================================================================================
#  TODO BEGIN install/upgrade specific code
# =====================================================================================
# TODO else
  # --------------------------------------
  # Clean install specific steps
  # --------------------------------------
  # First Time Install Processing. Save output to LOCAL_DIR

  # Copy the SSL cert files into place
  mkdir -p $ROOT_DIR/certs
  chmod 700 $ROOT_DIR/certs
  chown -R $INSTALL_USR:$INSTALL_USR $ROOT_DIR/certs
  chmod 600 $ROOT_DIR/certs/*.pem
  echo
  echo "===== Initialize server. Running tms_server --install as user: $INSTALL_USR"
  echo "========================================================================================="
  # Initialize the content of the install directory.
  INSTALL_INIT_CMD="$EXEC_FILE_DST --install --root-dir $ROOT_DIR"
  # We must run from the top of the source code checkout so the files under resources are available
  if [ "$TEST_MODE" != "true" ]; then
    su - $INSTALL_USR -c "cd $SRC_DIR; set -a; source $SVC_ENV_PATH; set +a; $INSTALL_INIT_CMD > ${LOCAL_DIR}/tms-install.out 2>&1"
    RET_CODE=$?
  else
    cd $SRC_DIR || exit 1
    $INSTALL_INIT_CMD > ${LOCAL_DIR}/tms-install.out 2>&1
    RET_CODE=$?
  fi
  if [ $RET_CODE -ne 0 ]; then
    echo "ERROR: Error running tms_server init. Command: $INSTALL_INIT_CMD"
    echo "       Please see output here: ${LOCAL_DIR}/tms-install.out"
    echo "Exiting ..."
    exit $RET_CODE
  fi
  chmod 400 $LOCAL_DIR/tms-install.out
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
    chmod 600 $ROOT_DIR/config/log4rs.yml
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
# TODO fi
# =====================================================================================
#  END install/upgrade specific code
# =====================================================================================

# Update version in install dir
echo "$VERS_NEW" > $VERS_FILE
chown $INSTALL_USR:$INSTALL_USR $VERS_FILE

# Start up the service
if [ "$TEST_MODE" != "true" ]; then
  echo
  echo "===== To start the TMS service, please run:"
  echo "  systemctl start tms_server"
  echo "========================================================================================="
fi

# Remove the temporary file
rm -f $TMP_FILE
# Switch back to current working directory of invoking user
cd "$RUN_DIR"
