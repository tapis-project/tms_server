#!/bin/bash
#
# Destructive uninstall of TMS DB and TMS server
#
echo "---------------------------------------------------"
echo " Destructive uninstnall of TMS DB and Server"
echo "---------------------------------------------------"
echo
echo "======================================================================================="
echo "======= WARNING ======= WARNING ======= WARNING ======= WARNING ======= WARNING ======="
echo "========================== THIS IS A DESTRUCTIVE OPERATION ============================"
echo "======================================================================================="
echo
read -p "WARNING DESTRUCTIVE UNINSTALL! Enter Y to continue: " resp
case $resp in
  [yY]* ) echo "Continuing ... " ;;
  *) echo "Uninstall cancelled. Exiting ... " ; exit 1 ;;
esac
echo
# Check for required env variables
if [ -z "${POSTGRES_PASSWORD}" ]; then
  echo "Please set env var POSTGRES_PASSWORD before running this script"
  exit 1
fi
echo "---------------------------------------------------"
echo " Dropping the TMS database"
echo "---------------------------------------------------"
echo
kubectl delete -f drop-db.yml
kubectl create configmap tms-drop-db-configmap --from-file drop-db-sh
kubectl create -f drop-db.yml
kubectl wait --timeout=200s --for=condition=complete job/tms-drop-db

echo "---------------------------------------------------"
echo " Undeploying TMS server and removing PVC"
echo "---------------------------------------------------"
echo
# Stop the service and delete the pvc
# Note: k8s should automatically wait for the pod to be removed before attempting to remove the pvc.
kubectl delete deployment tms-server pvc tms-server-vol

