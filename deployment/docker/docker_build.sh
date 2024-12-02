#!/bin/sh
#set -x

# ----------------------------------------------------------------
# Run this script from the tms_server/deployment/docker directory.
# ----------------------------------------------------------------

# Build docker image for tms_server. 
PrgName=$(basename "$0")
if [ $# -ne 1 ]; then 
    echo "Usage: $PrgName <docker tag>"
    echo "  where <docker tag> is the image version tag"
    echo "  and this script is run from the tms_server/deployment/docker directory"
    exit 1
fi

# Run docker image for the service
TAG=$1

# Build image
cd ../..
echo docker build -t "tapis/tms_server:"${TAG} -f deployment/docker/Dockerfile .
docker build -t "tapis/tms_server:"${TAG} -f deployment/docker/Dockerfile .
