#!/bin/bash
# Combined build/upgrade script for TMS Server
# Build and deploy the latest native version TMS Server
# This script must be run as root or run in --test mode.
#
# This script builds a release version and updates files in the install and root directories as needed.
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
#  - When running in non-test mode (i.e. as root), the source code is checked out under $HOME/tms_server.
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
# NOTE that in test mode we perform all of the TMS service related steps except for starting and stopping.
#   Set TMS_INSTALL_DIR=/tmp/tms_server which should allow any user to perform the service related config.

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
  su - $INSTALL_USR -c 'rustc --version'
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
if [ "$TEST_MODE" == "true" ]; then
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

# Determine new version
if [ "$TEST_MODE" == "true" ]; then
  VERS_NEW=$(cd $SRC_DIR; cargo pkgid | cut -d "#" -f2)
  RET_CODE=$?
else
  VERS_NEW=$(su - $INSTALL_USR -c 'cd $SRC_DIR; cargo pkgid | cut -d "#" -f2')
  RET_CODE=$?
fi
if [ $RET_CODE -ne 0 ]; then
  echo "Error determining new TMS version"
  echo "Exiting ..."
  exit $RET_CODE
fi

# Set paths to source and destination of tms_server executable
EXEC_FILE_SRC=$SRC_DIR/target/release/tms_server
EXEC_FILE_DST=$INSTALL_DIR/tms_server

# Set directory for service config file
SVC_CFG_DIR="$INSTALL_DIR/lib/systemd/system"

# Fill in some defaults needed by backup and upgrade migration
if [ -z "$TMS_DB_HOST" ]; then TMS_DB_HOST="localhost"; fi
if [ -z "$TMS_DB_PORT" ]; then TMS_DB_PORT="5432"; fi

# Set location of version file for installed version
VERS_FILE=$INSTALL_DIR/tms.version

# Output configuration
echo "******* Install / Upgrade Settings ************************"
echo "******* TMS Version: $VERS_NEW ********************************"
echo "   TEST_MODE=$TEST_MODE"
echo "   UPGRADE=$UPGRADE"
echo "   INSTALL_USR=$INSTALL_USR"
echo "   ROOT_DIR=$ROOT_DIR"
echo "   LOCAL_DIR=$LOCAL_DIR"
echo "   INSTALL_DIR=$INSTALL_DIR"
echo "   SVC_CFG_DIR=$SVC_CFG_DIR"
echo "   BAK_FILE_PATH=$BAK_FILE_PATH"
echo "   TMS_DB_HOST=$TMS_DB_HOST"
echo "   VERS_FILE=$VERS_FILE"
echo "***********************************************************"

# =====================================================================================
#  Perform install/upgrade specific checks related to previous install
#  Do this before we start the potentially time-consuming build process
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
echo "VERS_OLD=$VERS_OLD" >> $TMP_FILE
echo "VERS_NEW=$VERS_NEW" >> $TMP_FILE

# Construct second part of script
cat >> $TMP_FILE << EOB
echo "Upgrading TMS Server from version $VERS_OLD to version $VERS_NEW"
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
export TMS_DB_USER="tms"
export TMS_DB_DB_NAME="tmsdb"
export TMS_DB_HOST="$TMS_DB_HOST"
export TMS_DB_PORT="$TMS_DB_PORT"
export TMS_DB_USER_PASSWORD="$TMS_DB_USER_PASSWORD"
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
#  BEGIN install/upgrade specific code
# =====================================================================================
if [ "$UPGRADE" == "true" ]; then
  # --------------------------------------
  # Upgrade specific steps
  # --------------------------------------
  # Determine existing version
  if [ ! -f "$VERS_FILE" ]; then
    echo "Unable to determine existing TMS version. Cannot find file: $VERS_FILE"
    echo "Exiting ..."
    exit 1
  fi
  VERS_OLD=$(cat $VERS_FILE)

  # If there is a customizations directory then rename it to local to match layout as of 0.3.0
  if [ -d ${TMS_HOME}/tms_customizations ]; then
    echo
    echo "===== Moving customizations directory from ${TMS_HOME}/tms_customizations to $LOCAL_DIR"
    echo "========================================================================================="
    mv "${TMS_HOME}/tms_customizations" "$LOCAL_DIR"
  fi

  # Update migrations files.
  mv "$ROOT_DIR/migrations" "${ROOT_DIR}/migrations.bak_${BAK_TIMESTAMP}"
  cp -pr "${SRC_DIR}/resources/migrations" "${ROOT_DIR}/migrations"
  chmod 0700 "${ROOT_DIR}/migrations"
  chown $INSTALL_USR:$INSTALL_USR "${ROOT_DIR}/migrations"

  # Before updating version and starting up new tms_server, perform the migration from sqlite to postgres
  echo
  echo "===== Migrating DB from sqlite to postgres"
  echo "========================================================================================="
  # Fill in some defaults as needed before running migration
  TMS_TEST_MODE=$TEST_MODE
  TMS_USR=$INSTALL_USR
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
else
  # --------------------------------------
  # Clean install specific steps
  # --------------------------------------
  # First Time Install Processing. Save output to LOCAL_DIR
  echo
  echo "===== Initialize server. Running tms_server --install as user: $INSTALL_USR"
  echo "========================================================================================="
  # Initialize the content of the install directory.
  INSTALL_INIT_CMD="$EXEC_FILE_DST --install --root-dir $ROOT_DIR"
  # We must run from the top of the source code checkout so the files under resources are available
  if [ "$TEST_MODE" != "true" ]; then
    su - $INSTALL_USR -c "cd $SRC_DIR; source $SVC_ENV_PATH; $INSTALL_INIT_CMD > ${LOCAL_DIR}/tms-install.out 2>&1"
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
fi
# =====================================================================================
#  END install/upgrade specific code
# =====================================================================================

# Update version in install dir
echo "$VERS_NEW" > $VERS_FILE

# Start up the service
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
