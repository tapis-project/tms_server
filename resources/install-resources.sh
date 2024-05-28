#!/bin/bash

# This script copies files from the resources subtree to the specified 
# root data directory required by TMSS at runtime.

# Make sure we have at least 1 parameter.
if [ "$#" -eq  "0" ]
  then
    echo "Please supply the path to the target TMS data directory root."
    echo "This script copies files from subdirectories of the current directory "
    echo "to their corresponding subdirectories in supplied root data directory."
    echo
    echo "To create the root data directory, see: tms_server --help."
    echo
    echo "Example:  install-resources.sh ~/.tms"
    exit 1
fi

# Grab the specified target root directory.
rootdir=$1

# Copy certs files and assign permissions.
certsdir="${rootdir}/certs"
cp -p "certs/"* "$certsdir"
chmod 600 "$certsdir"/*

# Copy config files and assign permissions.
configdir="${rootdir}/config"
cp -p "config/"* "$configdir"
chmod 600 "$configdir"/*

# Copy migrations files and assign permissions.
migrationsdir="${rootdir}/migrations"
cp -p "migrations/"* "$migrationsdir"
chmod 600 "$migrationsdir"/*

# Substitute the default rootdir into log4rs.yml
# by replacing <PUT_YOUR_ROOTDIR_HERE> with the 
# user's $HOME value.
sed -i "s|<PUT_YOUR_ROOTDIR_HERE>|$HOME|g" "$configdir"/log4rs.yml 

