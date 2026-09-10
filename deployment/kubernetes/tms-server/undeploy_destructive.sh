#!/bin/bash
#
# Destructive uninstall of TMS DB and TMS server
#
echo "---------------------------------------------------"
echo " Destructive uninstall of TMS DB and Server"
echo "---------------------------------------------------"
echo
echo "======================================================================================="
echo "======= WARNING ======= WARNING ======= WARNING ======= WARNING ======= WARNING ======="
echo "========================== THIS IS A DESTRUCTIVE OPERATION ============================"
echo "======================================================================================="
echo
read -rp "WARNING DESTRUCTIVE UNINSTALL! Enter Y to continue: " resp
case $resp in
  [yY]* ) echo "Continuing ... " ;;
  *) echo "Uninstall cancelled. Exiting ... " ; exit 1 ;;
esac
echo
echo "---------------------------------------------------"
echo " Removing jobs"
echo "---------------------------------------------------"
echo
kubectl delete -f first-time-setup.yml
kubectl delete -f first-time-stage.yml
kubectl delete -f tms-server-sleep.yml
kubectl delete -f first-time-init-db.yml
kubectl delete -f tms-drop-db.yml
echo
echo "---------------------------------------------------"
echo " Dropping the TMS database"
echo "---------------------------------------------------"
echo
kubectl delete configmap tms-drop-db-configmap
kubectl delete -f tms-drop-db.yml
kubectl create configmap tms-drop-db-configmap --from-file tms-drop-db-sh
kubectl apply -f tms-drop-db.yml
kubectl wait --timeout=200s --for=condition=complete job/tms-drop-db

echo "---------------------------------------------------"
echo " Undeploying TMS server and removing PVC"
echo "---------------------------------------------------"
echo
# Stop the service and delete the pvc
# Note: k8s should automatically wait for the pod to be removed before attempting to remove the pvc.
kubectl delete deployment tms-server pvc tms-server-vol
