#!/bin/bash

# Ce script lance les instances singularity pour l'application web Mercure et la routine associée.
# Il doit être exécuté en tant qu'utilisateur 'hermes'.
# Ce script vérifie si les instances sont déjà en cours d'exécution avant de les démarrer.

[[ $USER != "hermes" ]] && echo "Ce script doit être exécuté en tant qu'utilisateur 'hermes'" && exit 1

MERCURE_SIF="/OPT/hermes/mercure.sif"

RUNNING_WEBAPP=$(singularity instance list | sed 1d | cut -d ' ' -f1 | grep mercure-webapp)

if [[ -z "$RUNNING_WEBAPP" ]];then
    singularity instance start $MERCURE_SIF -B /OPT/hermes -B /OPT/JOBS -B /OPT/pipelines mercure-webapp
    success=$?
    if [[ $success -ne 0 ]]; then
        echo "Échec du démarrage de l'instance mercure-webapp. La routine n'a pas été démarrée."
        exit $success
    else
        echo "Instance mercure-webapp démarrée avec succès"
    fi
else
    echo "Mercure webapp déjà en cours d'exécution"
fi

RUNNING_ROUTINE=$(singularity instance list | sed 1d | cut -d ' ' -f1 | grep mercure-routine)

if [[ -z "$RUNNING_ROUTINE" ]];then
    singularity instance start $MERCURE_SIF -B /OPT/hermes -B /OPT/JOBS -B /OPT/pipelines mercure-routine
    success=$?
    if [[ $success -ne 0 ]]; then
        echo "Échec du démarrage de l'instance mercure-routine"
        exit $success
    else
        echo "Instance mercure-routine démarrée avec succès"
    fi
else
    echo "Mercure routine déjà en cours d'exécution"
fi

