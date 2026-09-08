#!/usr/bin/env bash

# Run TMS image with a long sleep so we can examine how it looks inside the running container.

# The tag of the image to be run needs to be the first and only parameter.
PrgName=$(basename "$0")
if [ $# -ne 1 ]; then 
    echo "Usage: $PrgName <docker tag>"
    echo "  where <docker tag> is the image version tag"
    echo "E.g. $PrgName dev"
    exit 1
fi

# Assign the image tag
TAG=$1

# Start up a container that stays up for one week
docker run --name tms_sleep --user "tms" -d --rm --volume tms_server_vol:/home/tms tapis/tms_server:${TAG} \
    /bin/bash -c "sleep 6048000"
