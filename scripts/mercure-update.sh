#!/bin/bash

[[ $USER != "hermes" ]] && echo "Ce script doit être exécuté en tant qu'utilisateur 'hermes'" && exit 1

MERCURE_DIR="/OPT/mercure"

MERCURE_WEBAPP_NEW_SIF="$MERCURE_DIR/mercure-webapp-new.sif"
MERCURE_ROUTINE_NEW_SIF="$MERCURE_DIR/mercure-routine-new.sif"

MERCURE_WEBAPP_SIF="$MERCURE_DIR/mercure-webapp.sif"
MERCURE_ROUTINE_SIF="$MERCURE_DIR/mercure-routine.sif"

singularity instance stop mercure-webappd
singularity instance stop mercure-routined

mv $MERCURE_WEBAPP_NEW_SIF $MERCURE_WEBAPP_SIF
mv $MERCURE_ROUTINE_NEW_SIF $MERCURE_ROUTINE_SIF

"$MERCURE_DIR/mercure-run.sh"

