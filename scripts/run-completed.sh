#!/bin/bash
# Ce script reçoit deux arguments: le chemin absolu du répertoire de run brut, et le nom du séquenceur (qui fait partie normalement du chemin).
# Il doit retourner 1 si le dossier de run passé en argument n'est pas celui d'un run terminé. Autrement, le script retourne 0.
SEQ_DIR=$1
SEQ_NAME=$2
cd "$SEQ_DIR"

if [ -f "CopyComplete.txt" ] || [[ ${SEQ_NAME} == "M03869" && -f "RTAComplete.txt" && -f "CompletedJobInfo.xml" ]]; then
    exit 0
else
    exit 1
fi