#!/bin/bash
#
# Run the TMS first time setup script
#
echo "---------------------------------------------------"
echo " Running first time setup script for TMS Server"
echo "---------------------------------------------------"
echo

PrgName=$(basename "$0")
# Determine absolute path to location from which we are running and change to that directory.
RUN_DIR=$(pwd)
PRG_RELPATH=$(dirname "$0")
cd "$PRG_RELPATH"/. || exit
PRG_PATH=$(pwd)

# Check for required env variables
if [ -z "${POSTGRES_PASSWORD}" ]; then
  echo "Please set env var POSTGRES_PASSWORD before running this script"
  exit 1
fi
if [ -z "${TMS_DB_USER_PASSWORD}" ]; then
  echo "Please set env var TMS_DB_USER_PASSWORD before running this script"
  exit 1
fi

echo "---------------------------------------------------"
echo " Initializing the DB"
echo "---------------------------------------------------"
set -xv
kubectl delete -f first-time-init-db.yml
kubectl create configmap tms-first-time-init-db-configmap --from-file first-time-init-db-sh
kubectl create -f first-time-init-db.yml
kubectl wait --timeout=200s --for=condition=complete job/tms-first-time-init-db

echo "---------------------------------------------------"
echo " Running first time install"
echo "---------------------------------------------------"
kubectl delete -f first-time-install.yml
kubectl create configmap tms-first-time-install-configmap --from-file first-time-install-sh
kubectl create -f first-time-install.yml
kubectl wait --timeout=200s --for=condition=complete job/tms-first-time-install
