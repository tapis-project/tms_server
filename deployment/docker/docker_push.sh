#!/bin/sh
#set -x

# Build docker image for tms_server.
PrgName=$(basename "$0")
if [ $# -ne 1 ]; then 
    echo "Usage: $PrgName <docker tag>"
    echo "  where <docker tag> is the image version tag"
    exit 1
fi

# Assign the image tag
TAG=$1

# Publish image
echo "=================================================="
echo "To push: docker push "tapis/tms_server:"${TAG}"
echo "=================================================="
docker push "tapis/tms_server:"${TAG}"
