#!/usr/bin/env bash
#set -x

# This script should only be called AFTER docker_install.sh has successfully run.

# The tag of the image to be run needs to be the first and only parameter.
PrgName=$(basename "$0")
if [ $# -ne 1 ]; then 
    echo "Usage: $PrgName <docker tag>"
    echo "  where <docker tag> is the image version tag"
    exit 1
fi

# Assign the image tag
TAG=$1

# This script starts the tms_server in the background in a docker container under the user ID 
# that launches it.  The host's ~/tms-docker/tms_customizations directory is mounted into the 
# container and the persistent named volume, tms_docker_vol, contains the .tms directory that 
# the server uses during execution.  The container is removed when the server exits.
docker run --name tms_server_container --user $(id -u):$(id -g) -e HOME=/tms-root -p 3000:3000 -d --rm \
--volume tms_docker_vol:/tms-root \
--mount type=bind,source=${HOME}/tms-docker/tms_customizations,target=/tms-root/tms_customizations \
--volume="/etc/group:/etc/group:ro" \
--volume="/etc/passwd:/etc/passwd:ro" \
--volume="/etc/shadow:/etc/shadow:ro" \
tapis/tms_server:${TAG}