#!/bin/bash
#
# TMS Server first time setup and start script
#
echo "---------------------------------------------------"
echo " Running first time setup and start for TMS Server"
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

echo "---------------------------------------------------"
echo " Creating PVC"
echo "---------------------------------------------------"
kubectl apply -f pvc.yml

echo "---------------------------------------------------"
echo " Staging files in pvc"
echo "---------------------------------------------------"
kubectl delete -f first-time-stage.yml
kubectl apply -f first-time-stage.yml
kubectl wait --timeout=200s --for=condition=complete job/tms-first-time-stage

echo "---------------------------------------------------"
echo " Running first time install"
echo "---------------------------------------------------"
kubectl delete -f first-time-setup.yml
kubectl apply -f first-time-setup.yml
kubectl wait --timeout=200s --for=condition=complete job/tms-first-time-setup

echo "---------------------------------------------------"
echo " Starting up server for the first time"
echo "---------------------------------------------------"
kubectl delete -f deploy.yml
kubectl apply -f deploy.yml
kubectl wait --timeout=200s --for=condition=available deploy/tms-server

echo "---------------------------------------------------"
echo " Setting up ingress network access"
echo "---------------------------------------------------"
kubectl apply -f tms-server-ingress.yml

echo "---------------------------------------------------"
echo " Seeding initial config for tms-portal"
echo "---------------------------------------------------"
TMS_PORTAL_SQL_FILE="$HOME/tms-portal/init.sql"
if [ -r "$TMS_PORTAL_SQL_FILE" ]; then
  # Seed config for tms-portal from file $HOME/tms-portal/init.sql
  cat "$TMS_PORTAL_SQL_FILE" | kubectl exec -i deploy/tms-postgres-18 -- psql -U tms tmsdb
else
  echo "NOTE: TMS Portal init sql file not found. Initial seeding for tms-portal will not be done"
  echo "File: $TMS_PORTAL_SQL_FILE Portal init sql file not found. Initial seeding for tms-portal will not be done"
fi
