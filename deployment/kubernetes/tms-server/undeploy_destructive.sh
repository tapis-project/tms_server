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
echo "======================================================================================="
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
kubectl delete -f first-time-setup.yml 2>/dev/null
kubectl delete -f first-time-stage.yml 2>/dev/null
kubectl delete -f tms-server-sleep.yml 2>/dev/null
kubectl delete -f first-time-init-db.yml 2>/dev/null
kubectl delete -f tms-drop-db.yml 2>/dev/null
echo
echo "---------------------------------------------------"
echo " Dropping the TMS database"
echo "---------------------------------------------------"
echo
kubectl delete configmap tms-drop-db-configmap
kubectl delete -f tms-drop-db.yml 2>/dev/null
kubectl create configmap tms-drop-db-configmap --from-file tms-drop-db-sh
kubectl apply -f tms-drop-db.yml
kubectl wait --timeout=200s --for=condition=complete job/tms-drop-db

# Bring down tms-portal if we have a deploy file for it
TMS_PORTAL_DEPLOY="$HOME/tms-portal/deployment/deploy.yml"
if [ -f "$TMS_PORTAL_DEPLOY" ]; then
 echo "---------------------------------------------------"
 echo " Undeploying TMS portal"
 echo "---------------------------------------------------"
  kubectl apply -f $TMS_PORTAL_DEPLOY
  kubectl wait --for=delete deploy/tms-portal
else
 echo "---------------------------------------------------"
 echo " Skipping undeploy of TMS portal"
 echo "---------------------------------------------------"
fi
echo
echo "---------------------------------------------------"
echo " Undeploying TMS server and removing PVC"
echo "---------------------------------------------------"
echo
kubectl delete -f deploy.yml
kubectl wait --for=delete deploy/tms-server
kubectl delete -f pvc.yml
