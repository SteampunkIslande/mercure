#!/bin/bash

# Compresser les logs de slurm dans une archive
# Le master log se trouve dans la variable d'environnement HG_LOG_FILE

# Dans ce fichier de log, on peut extraire les logs de chaque job, grâce au nom du job slurm:
# Pour retrouver le nom du job slurm, on peut chercher la ligne:
# SLURM run ID:\s+(.+)


# Celle-ci peut apparaître plusieurs fois, et permet de sélectionner tous les jobs slurm lancés par le script.
# Pour chaque occurrence, on peut extraire les logs correspondants via sacct:
# sacct --name <SLURM run ID> --format=JobID,State,ExitCode,Start,End,Elapsed,MaxRSS

echo "## STEP Exécution du script de post-traitement... Extraction des logs SLURM."

if [[ -z "$HG_LOG_FILE" ]]; then
    echo "## WARNING: La variable d'environnement HG_LOG_FILE n'est pas définie."
else
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
fi

