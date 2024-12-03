#!/usr/bin/env bash
#set -x

# This script should be run one time on the user account under which tms_server
# will execute.  It creates and initializes directories on the host and in 
# persistent named volumes used by the server.

# The tag of the image to be run needs to be the first and only parameter.
PrgName=$(basename "$0")
if [ $# -ne 1 ]; then 
    echo "Usage: $PrgName <docker tag>"
    echo "  where <docker tag> is the image version tag"
    exit 1
fi

# Assign the image tag
TAG=$1

# Create the customizations directory if it doesn't exist and make it private.
# This directory is written to by tms_server when the server is started with 
# the --install option.  The file written, tms-install.out, contains a record
# of the installation process and the administrator user IDs and passwords for
# the "default" and "test" tenants.  These administrator credentials are only
# exposed in tms-install.out and should be kept secret.
mkdir -p ~/tms-docker/tms_customizations
chmod 700 ~/tms-docker
chmod 700 ~/tms-docker/tms_customizations

# Run tms_server in installation mode.  This command establishes the current
# user as the user ID under which the server will run; creates the named
# volume "tms_docker_vol" and initializes it contents; bind mounts the 
# ~/tms-docker/tms_customizations directory; volume mounts a number of linux 
# configuration files read-only; and outputs its results to 
# ~/tms-docker/tms_customizations/tms-install.out.
#
# The tms_server container is removed when the program exits, but its state is
# saved in the named "tms_docker_vol" volume.  When the server is restarted, the 
# saved state will be used.
#
# The volume mount of the host's tms-docker directory to the container's tms home 
# directory creates a named volume that outlives the container and can be written 
# to from outside the container  using "docker cp".
#
# The Bind mount of the host's tms_customizations directory over the container's 
# tms_customizations directory.  The bind mount obscures any pre-existing content 
# that might be in the container's directory, but it allows r/w from both the host 
# and the container.
#
# Note: The use of ${HOME} rather than ~ is necessary due to docker's less than 
#       perfect test for absolute paths.
docker run --name tms_server_container --user $(id -u):$(id -g) -e HOME=/tms-root --rm \
--volume tms_docker_vol:/tms-root \
--mount type=bind,source=${HOME}/tms-docker/tms_customizations,target=/tms-root/tms_customizations \
--volume="/etc/group:/etc/group:ro" \
--volume="/etc/passwd:/etc/passwd:ro" \
--volume="/etc/shadow:/etc/shadow:ro" \
tapis/tms_server:${TAG} \
/bin/bash -c "/tms-root/tms_server/tms_server --root-dir /tms-root/.tms --install > \
/tms-root/tms_customizations/tms-install.out 2>&1"

# Make the installation output file private to the user under which tms_server runs.
# This file contains administrator credentials, so it needs to be guarded or moved.
chmod 600 ~/tms-docker/tms_customizations/tms-install.out