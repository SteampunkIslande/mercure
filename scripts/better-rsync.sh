#!/bin/bash

cd "$PIPELINES_DIR/copie"

FROM=$1
TO=$2

job_name=copie-$(date +%s)

echo "SLURM run ID: $job_name"

srun --job-name="$job_name" --mem=4G rsync -a --info=progress2 "$FROM" "$TO" | tr '\r' '\n' | sed -r 's/.+\s([0-9]+)%.+/\1 of 100 (\1%) done/gm'
res=$?

if [ $res -ne 0 ]; then
	echo "## ERROR Erreur lors de la copie avec rsync"
	exit $res
else
	echo "Copie terminée"
fi
