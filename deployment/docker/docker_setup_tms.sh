#!/usr/bin/env bash
# This script should be run one time on the user account under which tms_server will execute.
# It creates and initializes directories on the host and in persistent named volumes used by the server.
# The tag of the image to be run needs to be the first and only parameter.
PrgName=$(basename "$0")
if [ $# -ne 1 ]; then 
    echo "Usage: $PrgName <docker tag>"
    echo "  where <docker tag> is the image version tag"
    echo "E.g. $PrgName dev"
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


# TODO Update comments
# Run tms_server in installation mode. This command establishes the current user as the user ID under which the server
# will run; creates the named volume "tms_server_vol" and initializes it contents; bind mounts
# the ~/tms-docker/tms_local directory; volume mounts a number of linux configuration files read-only; and outputs its
# results to ~/tms-docker/tms_local/tms-install.out.
#
# The tms_server container is removed when the program exits, but its state is saved in the named "tms_server_vol"
# volume. When the server is restarted, the saved state will be used.
#
# The volume mount of the host's tms-docker directory to the container's tms home directory creates a named volume that
# outlives the container and can be written to from outside the container using "docker cp".
#
# The bind mount of the host's tms_local directory is done over the container's tms_local directory.
# The bind mount obscures any pre-existing content that might be in the container's directory, but it allows r/w from
# both the host and the container.
#
# Note: The use of ${HOME} rather than ~ is necessary due to docker's less than perfect test for absolute paths.
#docker run --name tms_server --user "$(id -u)":"$(id -g)" -e HOME=/tms-root --network="host" --rm \
#set -xv
#docker run --name tms_server --user "$(id -u)":"$(id -g)" -e HOME=/tms-root --network="host" --rm \
#   -e TMS_DB_HOST=$TMS_DB_HOST -e TMS_DB_PORT=$TMS_DB_PORT -e TMS_DB_USER_PASSWORD=$TMS_DB_USER_PASSWORD \
#  --volume tms_server_vol:/tms-root \
#  --mount type=bind,source=${HOME}/tms-docker,target=/tms-root \
#  --volume="/etc/group:/etc/group:ro" \
#  --volume="/etc/passwd:/etc/passwd:ro" \
#  --volume="/etc/shadow:/etc/shadow:ro" \
#  tapis/tms_server:${TAG} \
#  /bin/bash -c "/tms-root/tms_server/tms_server --root-dir /tms-root/tms_server --install > \
#  /tms-root/tms_local/tms-install.out 2>&1"
docker run --name tms_server --user "tms" --network="host" --rm \
   -e TMS_DB_HOST=$TMS_DB_HOST -e TMS_DB_PORT=$TMS_DB_PORT -e TMS_DB_USER_PASSWORD=$TMS_DB_USER_PASSWORD \
  --volume tms_server_vol:/home/tms \
  tapis/tms_server:${TAG} \
  /bin/bash -c "cd /home/tms/tms_server; ./tms_server --root-dir /home/tms/tms --install > /home/tms/tms_local/tms-install.out 2>&1"

# TODO remove or do this via a docker run command?
## Make the installation output file private to the user under which tms_server runs.
## This file contains administrator credentials, so it needs to be guarded or moved.
#chmod 600 ~/tms-docker/tms_local/tms-install.out
