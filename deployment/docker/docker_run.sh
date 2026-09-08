#!/usr/bin/env bash

# This script should only be called AFTER docker_setup_tms.sh has successfully run.

# The tag of the image to be run needs to be the first and only parameter.
PrgName=$(basename "$0")
if [ $# -ne 1 ]; then 
    echo "Usage: $PrgName <docker tag>"
    echo "  where <docker tag> is the image version tag"
    echo "For example $PrgName dev"
    exit 1
fi
# Check that all required env variables are set
FAILED=false
#env_list="POSTGRES_PASSWORD TMS_DB_USER_PASSWORD"
env_list="TMS_DB_USER_PASSWORD"
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
TMS_DB_HOST="${TMS_DB_HOST:-127.0.0.1}"
TMS_DB_PORT="${TMS_DB_PORT:-5432}"

# Assign the image tag
TAG=$1

# This script starts the tms_server in the background in a docker container under the user ID that launches it.
# The container persistent named volume, tms_server_vol, contains all the files used at runtime.
# The container is removed when the server exits.
docker run --name tms_server --user "tms" --network="host" -d --rm \
  -e TMS_DB_HOST=$TMS_DB_HOST -e TMS_DB_PORT=$TMS_DB_PORT -e TMS_DB_USER_PASSWORD=$TMS_DB_USER_PASSWORD \
  --volume tms_server_vol:/home/tms \
  tapis/tms_server:${TAG}
