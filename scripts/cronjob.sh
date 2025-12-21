#!/bin/bash

# /OPT/JOBS en production
JOBS_DIR=/home/charles/mercure-arena/OPT/JOBS

cd $JOBS_DIR/TODO

job=$(find . -maxdepth 1 -name "*.sh" | head -n1)

# Pas de job
[[ -z $job ]] && { exit 0; }

job_name=$(date +%s)-$(basename ${job%.sh})

mv $job $JOBS_DIR/RUNNING/${job_name}.sh

echo -e "BEGIN_SCRIPT\n----" >>$JOBS_DIR/LOGS/${job_name}.log 2>&1
cat $JOBS_DIR/RUNNING/${job_name}.sh >>$JOBS_DIR/LOGS/${job_name}.log 2>&1
echo -e "END_SCRIPT\n----" >>$JOBS_DIR/LOGS/${job_name}.log 2>&1
stdbuf -oL $JOBS_DIR/RUNNING/${job_name}.sh >>$JOBS_DIR/LOGS/${job_name}.log 2>&1

RESULT=$?

if [[ $RESULT -eq 0 ]]; then
    mv $JOBS_DIR/RUNNING/${job_name}.sh $JOBS_DIR/DONE
else
    mv $JOBS_DIR/RUNNING/${job_name}.sh $JOBS_DIR/FAILS
fi
