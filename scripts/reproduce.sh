#!/bin/bash

# Prend deux arguments: le chemin vers le fichier de log de l'analyse à reproduire et le chemin vers le script de reproduction à générer

[[ $# -ne 2 ]] && { echo "Usage: $0 /chemin/vers/fichier.log /chemin/vers/script/de/repro.sh"; exit 1; }

LOG_FILE="$1"
REPRO_SCRIPT="$2"

[[ -f $LOG_FILE ]] || { echo "Erreur: le fichier de log $LOG_FILE est introuvable"; exit 1; }

# Extraire le script de reproduction du fichier de log

sed -n '/^BEGIN_REPRO_SCRIPT$/,/^END_REPRO_SCRIPT$/p' "$LOG_FILE" | sed '1d;$d' > "$REPRO_SCRIPT"

chmod +x "$REPRO_SCRIPT"

echo "Script de reproduction généré: $REPRO_SCRIPT. Avant de l'exécuter, assurez-vous que les variables d'environnement nécessaires ont des valeurs appropriées."