#!/usr/bin/env bash

# The tag of the image to be run needs to be the first and only parameter.
PrgName=$(basename "$0")
if [ $# -ne 1 ]; then 
    echo "Usage: $PrgName <docker tag>"
    echo "  where <docker tag> is the image version tag"
    echo "  and this script is run from the tms_server/deployment/docker directory"
    exit 1
fi

# Assign the image tag
TAG=$1

# Run docker image for the service
TMS_VERSION=${TAG} TMS_UID=$(id -u) TMS_GID=$(id -g) docker compose up -d
