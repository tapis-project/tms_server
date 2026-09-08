#!/bin/bash
# Script to uninstall and re-install a docker image
# Requirements:
#  * Note that is assumes the image to use is tapis/tms_server:dev
#  * The seeding sql for the portal tables (providers, etc) must be at:
#           $HOME/tms_portal_seed_local.sql
#  * The following environment variables should be set, the first 2 are required:
#    - POSTGRES_PASSWORD (required)
#    - TMS_DB_USER_PASSWORD (required)
#    - TMS_DB_HOST (optional, default = localhost)
#    - TMS_DB_PORT (optional, default = 5432)
DTAG="dev"
PrgName=$(basename "$0")
# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

RUN_PATH="$PRG_PATH/../deployment/docker"
MIGRATION_DIR="$PRG_PATH/../resources/migrations"
PORTAL_SEED_DATA="$HOME/tms_portal_seed_local.sql"

# Exit if a command returns status different from 0
set -o errexit
# An unset variable is an error (avoids silently continuing after a typo in a name)
set -o nounset
# If any of the components of a pipe fails, then the pipe fails
set -o pipefail

# Set up to run psql
TMS_DB_HOST="${TMS_DB_HOST:-localhost}"
TMS_DB_PORT="${TMS_DB_PORT:-5432}"
TMS_DB_USER="${TMS_DB_USER:-tms}"
DB_NAME="${TMS_DB_DB_NAME:-tmsdb}"
if [ -z "${POSTGRES_PASSWORD}" ]; then
  echo "Please set env variables POSTGRES_PASSWORD, TMS_DB_USER_PASSWORD before running this script"
  exit 1
fi
if [ -z "${TMS_DB_USER_PASSWORD}" ]; then
  echo "Please set env var POSTGRES_PASSWORD, TMS_DB_USER_PASSWORD before running this script"
  exit 1
fi

DB_URL="postgres://${TMS_DB_USER}:${TMS_DB_USER_PASSWORD}@${TMS_DB_HOST}:${TMS_DB_PORT}/${DB_NAME}"

PSQL_CMD="psql $DB_URL"

# Uninstall
echo
echo "=================================="
echo " Running uninstall"
echo "=================================="
$RUN_PATH/docker_uninstall.sh

# Remove old image. Use -f to avoid non-zero exit code if image does not exist
echo
echo "=================================="
echo " Removing old docker images"
echo "=================================="
docker rmi -f tapis/tms_server:$DTAG

# Build new image
echo
echo "=================================="
echo " Building new docker image"
echo "=================================="
$RUN_PATH/docker_build.sh $DTAG

# Init DB tables
echo
echo "=================================="
echo " Initializing DB tables"
echo "=================================="
cargo install sqlx-cli
sqlx migrate run --database-url ${DB_URL} --source ${MIGRATION_DIR}

# Seed test data
echo
echo "=================================="
echo " Seeding test data"
echo "=================================="
echo "$PSQL_CMD < $PORTAL_SEED_DATA"
$PSQL_CMD < $PORTAL_SEED_DATA

# Set up tms_server
echo
echo "=================================="
echo " Setting up tms_server"
echo "=================================="
$RUN_PATH/docker_setup_tms.sh $DTAG

# Start a long running container to allow us to see the volume
echo
echo "=================================="
echo " Starting container in sleep mode"
echo "=================================="
$RUN_PATH/docker_sleep_tms.sh $DTAG

# Show result of the setup step
echo
echo "================================================================================================================="
echo " Echoing output of tms-install.out to screen."
echo "================================================================================================================="
docker run --rm -v tms_server_vol:/tms_vol alpine cat /tms_vol/tms_local/tms-install.out
echo "================================================================================================================="
