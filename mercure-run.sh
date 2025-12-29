#!/bin/bash

# Ce script lance les instances singularity pour l'application web Mercure et la routine associée.
# Il doit être exécuté en tant qu'utilisateur 'hermes'.
# Ce script vérifie si les instances sont déjà en cours d'exécution avant de les démarrer.

[[ $USER != "hermes" ]] && echo "Ce script doit être exécuté en tant qu'utilisateur 'hermes'" && exit 1

MERCURE_WEBAPP_SIF="/OPT/mercure/mercure-webapp.sif"
MERCURE_ROUTINE_SIF="/OPT/mercure/mercure-routine.sif"

JOBS_DIR="/OPT/JOBS"
PIPELINES_DIR="/OPT/pipelines"

RUNNING_WEBAPP=$(singularity instance list | sed 1d | cut -d ' ' -f1 | grep mercure-webappd)

if [[ -z "$RUNNING_WEBAPP" ]];then
    singularity instance start $MERCURE_WEBAPP_SIF -B /OPT/mercure -B $JOBS_DIR -B $PIPELINES_DIR mercure-webappd
    success=$?
    if [[ $success -ne 0 ]]; then
        echo "Échec du démarrage de l'instance mercure-webappd. La routine n'a pas été démarrée."
        exit $success
    else
        echo "Instance mercure-webappd démarrée avec succès"
    fi
else
    echo "mercure-webappd déjà en cours d'exécution"
fi

RUNNING_ROUTINE=$(singularity instance list | sed 1d | cut -d ' ' -f1 | grep mercure-routined)

if [[ -z "$RUNNING_ROUTINE" ]];then
    singularity instance start $MERCURE_ROUTINE_SIF -B /OPT/mercure -B $JOBS_DIR -B $PIPELINES_DIR mercure-routined
    success=$?
    if [[ $success -ne 0 ]]; then
        echo "Échec du démarrage de l'instance mercure-routined"
        exit $success
    else
        echo "Instance mercure-routined démarrée avec succès"
    fi
else
    echo "mercure-routined déjà en cours d'exécution"
fi

