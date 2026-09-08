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

echo "---------------------------------------------------"
echo " Initializing the DB"
echo "---------------------------------------------------"
kubectl delete configmap tms-first-time-init-db-configmap
kubectl delete -f first-time-init-db.yml
kubectl create configmap tms-first-time-init-db-configmap --from-file first-time-init-db-sh
kubectl apply -f first-time-init-db.yml
kubectl wait --timeout=200s --for=condition=complete job/tms-first-time-init-db

# TODO/TBD
echo "---------------------------------------------------"
echo " Staging files in pvc"
echo "---------------------------------------------------"
kubectl delete -f first-time-stage.yml
kubectl apply -f first-time-stage.yml
kubectl wait --timeout=200s --for=condition=complete job/tms-first-time-stage

echo "---------------------------------------------------"
echo " Running first time install"
echo "---------------------------------------------------"
kubectl delete -f first-time-install.yml
kubectl apply -f first-time-install.yml
kubectl wait --timeout=200s --for=condition=complete job/tms-first-time-install
