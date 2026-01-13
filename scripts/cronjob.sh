#!/bin/bash

# /OPT/JOBS en production
JOBS_DIR=/home/charles/mercure-arena/OPT/JOBS

# /OPT/mercure/reproductibility-script.sh en production
REPRODUCIBILITY_SCRIPT="/home/charles/mercure/scripts/reproductibility-script.sh"

[[ -f $REPRODUCIBILITY_SCRIPT ]] || { echo "Erreur: le script de reproductibilité $REPRODUCIBILITY_SCRIPT est introuvable, impossible de lancer l'analyse"; exit 1; }

cd "$JOBS_DIR/TODO"

job=$(find . -maxdepth 1 -name '*.sh' | head -n1)

# Pas de job
[[ -z $job ]] && { exit 0; }

job_name="$(date +%s)-$(basename ${job%.sh})"

export HG_LOG_FILE="$JOBS_DIR/LOGS/${job_name}.log"

mv "$job" "$JOBS_DIR/RUNNING/${job_name}.sh"
echo -e "BEGIN_REPRO_SCRIPT\n----" >>"$HG_LOG_FILE" 2>&1

# Affiche le script de reproductibilité dans le log, ainsi que le script que l'on s'apprête à lancer
# Le but est de pouvoir relancer exactement le même job plus tard, juste à partir du log
# Tout le contenu entre les deux marqueurs BEGIN_REPRO_SCRIPT et END_REPRO_SCRIPT est un script à part entière, que l'utilisateur peut adapter (si certaines variables d'environnement doivent changer) et relancer
echo "#!/bin/bash" >>"$HG_LOG_FILE" 2>&1
echo "export REPRO_COMMIT=$(git rev-list -n 1 HEAD)" >>"$HG_LOG_FILE" 2>&1
cat $REPRODUCIBILITY_SCRIPT <( cat "$JOBS_DIR/RUNNING/${job_name}.sh" | sed 1d ) | sed '/export PIPELINE_DIR=/d' >>"$HG_LOG_FILE" 2>&1
echo -e "END_REPRO_SCRIPT\n----" >>"$HG_LOG_FILE" 2>&1

stdbuf -oL "$JOBS_DIR/RUNNING/${job_name}.sh" >>"$HG_LOG_FILE" 2>&1
RESULT=$?

if [[ $RESULT -eq 0 ]]; then
    mv $JOBS_DIR/RUNNING/${job_name}.sh $JOBS_DIR/DONE
else
    mv $JOBS_DIR/RUNNING/${job_name}.sh $JOBS_DIR/FAILS
fi
