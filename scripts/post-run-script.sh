#!/bin/bash

# Compresser les logs de slurm dans une archive
# Le master log se trouve dans la variable d'environnement HG_LOG_FILE

# Dans ce fichier de log, on peut extraire les logs de chaque job, grâce au nom du job slurm:
# Pour retrouver le nom du job slurm, on peut chercher la ligne:
# SLURM run ID:\s+(.+)


# Celle-ci peut apparaître plusieurs fois, et permet de sélectionner tous les jobs slurm lancés par le script.
# Pour chaque occurrence, on peut extraire les logs correspondants via sacct:
# sacct --name <SLURM run ID> --format=JobID,State,ExitCode,Start,End,Elapsed,MaxRSS

# Liste des variables d'environnement disponibles:
# HG_LOG_FILE : chemin vers le log principal (exécution du script déposé dans JOBS/TODO et lancé par cronjob.sh)
# OUTDIR: répertoire de travail du pipeline
# INDIR: répertoire d'entrée du pipeline
# HG_ATTEMPT_ID: numéro de la tentative d'exécution du pipeline
# HG_RUN_ID: numéro du run (peut regrouper plusieurs tentatives)
# HG_ATTEMPT_STATUS: statut de la tentative (`SUCCESS` ou `FAILED`)

# Toutes les variables d'environnement définies par le formulaire sont également disponibles.

cd "$OUTDIR"
LOG_BASENAME=$(basename "$HG_LOG_FILE")
# Pour rechercher plus facilement avec sacct, si on prend `basename $HG_LOG_FILE`, ce dernier commence par un timestamp suivi d'un tiret et du nom du script.
JOB_START_TIMESTAMP=${LOG_BASENAME%%-*}
# Extraire les IDs de jobs SLURM depuis le log: à chaque fois qu'on rencontre une ligne "SLURM run ID: <ID>", on récupère l'ID, et on recherche les infos sur ce(s) jobs avec sacct.
SLURM_JOB_IDS=$(grep -oP 'SLURM run ID:\s+(\S+)' "$HG_LOG_FILE" | awk '{print $4}')
JOB_NUMBER=0
for JOB_ID in $SLURM_JOB_IDS; do
    JOB_NUMBER=$((JOB_NUMBER + 1))
    sacct --name "$JOB_ID" -P -o JobID,JobName,Account,NodeList,AllocCPUs,MaxRSS,Elapsed,State -S $(date -d @"$JOB_START_TIMESTAMP" +%Y-%m-%d) | grep '\.[0-9]' > "usage_memory_STEP_${JOB_NUMBER}.csv" 2>&1
done

total_time="$(( $(date +%s) - JOB_START_TIMESTAMP ))"
total_d=$((total_time / 86400))
total_h=$(( (total_time % 86400) / 3600))
total_m=$(( (total_time % 3600) / 60))
total_s=$(( total_time % 60 ))

echo "## INFO Temps d'exécution: $total_d jours, $total_h heures, $total_m minutes, $total_s secondes" >> "$HG_LOG_FILE" 2>&1
