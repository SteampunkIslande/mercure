# Script de reproduction - crée un nouveau PIPELINE_DIR avec l'état exact du dépôt git
# Pour le lancer, se placer dans un répertoire vide et exécuter ce script avec pour seul argument un chemin vers le dépôt git de référence

[[ $# -ne 2 ]] && { echo "Usage: $0 /path/to/origin /path/to/new_pipeline_dir"; exit 1; }

REMOTE_REPO_PATH="$1"
NEW_PIPELINE_DIR="$2"

[[ -d $NEW_PIPELINE_DIR ]] && { echo "Erreur: le répertoire $NEW_PIPELINE_DIR existe déjà"; exit 1; }

[[ -z $REPRO_COMMIT ]] && { echo "Erreur: la variable d'environnement REPRO_COMMIT n'est pas définie"; exit 1; }

git clone "$REMOTE_REPO_PATH" "$NEW_PIPELINE_DIR"
cd "$NEW_PIPELINE_DIR" || { echo "Erreur: impossible de se placer dans $NEW_PIPELINE_DIR"; exit 1; }
git reset --hard "$REPRO_COMMIT"

# Redéfinir PIPELINE_DIR pour pointer vers le nouveau répertoire
export PIPELINE_DIR="$NEW_PIPELINE_DIR"

# Ci-dessous, le script tel qu'il a été exécuté à l'origine (à l'exception de la ligne définissant PIPELINE_DIR)