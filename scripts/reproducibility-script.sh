# Script de reproduction - crée un nouveau PIPELINE_DIR avec l'état exact du dépôt git
# Pour le lancer, se placer dans un répertoire vide et exécuter ce script avec pour seul argument un chemin vers le dépôt git de référence

[[ $# -ne 1 ]] && { echo "Usage: $0 /path/to/repo"; exit 1; }

TIMESTAMP_REPRO=$(date +%s)
NEW_PIPELINE_DIR="${TIMESTAMP_REPRO}_repro"

git clone "$1" "$NEW_PIPELINE_DIR"
cd "$NEW_PIPELINE_DIR" || { echo "Erreur: impossible de se placer dans $NEW_PIPELINE_DIR"; exit 1; }
git reset --hard "$REPRO_COMMIT"

# Redéfinir PIPELINE_DIR pour pointer vers le nouveau répertoire
export PIPELINE_DIR="$NEW_PIPELINE_DIR"

# Ci-dessous, le script tel qu'il a été exécuté à l'origine