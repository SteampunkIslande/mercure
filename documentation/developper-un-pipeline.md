# Développement d'un pipeline

Ce document explique comment développer des pipelines pour Mercure, en détaillant les structures, les bonnes pratiques et les exemples concrets.

---

## Pipeline de copie

Bien que les cas d'usage d'un pipeline dédié uniquement à la copie soient limités, il est possible d'en créer un avec un launcher spécifique. Ce dernier utilise l'action associée située dans `copie/actions/copie-1.0.0.sh`.

### Structure du pipeline

```
copie
├── actions
│   └── copie-1.0.0.sh
└── launchers
    └── copie-1.0.0.sh
```

> **Important** : Assurez-vous que les scripts sont exécutables (`chmod +x`).

---

### Action de copie (`copie/actions/copie-1.0.0.sh`)

```bash
#!/bin/bash

cd "$PIPELINES_DIR/copie"

FROM=$1
TO=$2

job_name=copie-$(date +%s)

echo "SLURM run ID: $job_name"

srun --job-name="$job_name" --mem=4G rsync -a --info=progress2 "$FROM" "$TO" | \
    tr '\r' '\n' | \
    sed -r 's/.+\s([0-9]+)%.+/\1 of 100 (\1%) done/gm'
res=$?

if [ $res -ne 0 ]; then
    echo "## ERROR Erreur lors de la copie avec rsync (code de retour: $res)"
    exit $res
else
    echo "Copie terminée"
fi
```

#### Points clés de l'action

- **Arguments** : Le script utilise les arguments `1` et `2` pour définir les variables `FROM` et `TO`.
- **Nom du job SLURM** : Un identifiant unique est généré pour suivre l'exécution dans Mercure.
- **Exécution via SLURM** : Toujours utiliser `srun` pour exécuter une commande.
- **Communication avec Mercure** :
  - La progression est indiquée avec le format `X of 100 (X%) done`, compatible avec l'interface de Mercure.
  - Les erreurs sont signalées avec le préfixe `## ERROR`.
- **Gestion des erreurs** : En cas d'échec, retourner un code non nul pour arrêter le launcher (qui s'exécute en mode strict avec `set -euo pipefail`).

---

### Launcher (`copie/launchers/default.sh`)

```bash
#!/bin/bash

## COPIE_DEST : Destination de la copie, au format attendu par rsync (ex: user@host:/dossier/depot)

cd "$PIPELINE_DIR/copie"

echo "##STEP Étape 1/1: copie de $INDIR vers $COPIE_DEST"
./actions/copie-1.0.0.sh "$INDIR" "$COPIE_DEST"
```

#### Rôle du launcher

- **Orchestration** : Regroupe et exécute les tâches.
- **Communication** : Informe Mercure de l'étape en cours avec `##STEP` suivi du numéro de l'étape.

> **Résumé** : Un pipeline de copie simple comprend 1 launcher utilisant 1 action.

---

## Pipeline de démultiplexage

### Structure du pipeline

```
demul
├── actions
│   └── demul-1.0.0.sh
└── launchers
    ├── default.sh
    └── demul-copie.sh
```

---

### Action de démultiplexage (`demul/actions/demul-1.0.0.sh`)

```bash
#!/bin/bash

demul_job_name=demul-$(date +%s)

# Permet de suivre l'évolution du run via l'interface Mercure
echo "SLURM run ID: $demul_job_name"

# Lancement du job SLURM pour le démultiplexage avec bcl-convert
srun --job-name=$demul_job_name --output=$OUTDIR/demul-%j.out --mem=64G \
    singularity exec /SINGULARITIES/bcl-convert.sif bcl-convert \
        --sample-sheet $INDIR/adn.csv \
        --output-dir $OUTDIR/fastq \
        --bcl-input-dir $INDIR \
        --no-lane-splitting true
```

---

### Launcher par défaut (`demul/launchers/default.sh`)

```bash
#!/bin/bash

cd "$PIPELINE_DIR/demul"

echo "Étape 1/1: Démultiplexage de $INDIR dans $OUTDIR"

./actions/demul-1.0.0.sh
```

---

## Pipeline de démultiplexage et copie

### Launcher combiné (`demul/launchers/demul-copie.sh`)

```bash
#!/bin/bash

## COPIE_DEST : Destination de la copie, au format attendu par rsync (ex: user@host:/dossier/depot)

[[ -z $COPIE_DEST ]] && { echo "## ERROR COPIE_DEST est obligatoire"; exit 1; };

# Étape 1 : Démultiplexage
cd "$PIPELINE_DIR/demul"
echo "Étape 1/2: Démultiplexage de $INDIR dans $OUTDIR"
./actions/demul-1.0.0.sh

# Étape 2 : Copie
cd "$PIPELINE_DIR/copie"
echo "Étape 2/2: Copie de $OUTDIR dans $COPIE_DEST"
./actions/copie-1.0.0.sh $OUTDIR $COPIE_DEST
```

---

## Utilisation de Snakemake dans les actions

Pour simplifier la gestion des jobs SLURM et la communication avec Mercure, il est possible d'utiliser Snakemake dans les actions. Cela évite d'écrire manuellement les commandes `echo` pour le suivi de progression.

### Exemple d'actions Snakemake

#### Action à forte demande de mémoire (`vidjil-v1/actions/vidjil-high-memory-1.0.0.sh`)

```bash
cd "$PIPELINE_DIR/vidjil-v1"
snakemake --workflow-profile high_memory -d $OUTDIR
```

#### Action à faible demande de mémoire (`vidjil-v1/actions/vidjil-low-memory-1.0.0.sh`)

```bash
cd "$PIPELINE_DIR/vidjil-v1"
snakemake --workflow-profile low_memory -d $OUTDIR
```

---

### Launcher avec sélection de profil (`vidjil-v1/launchers/default.sh`)

```bash
#!/bin/bash

## HIGH_MEMORY : Variable optionnelle (OUI ou NON, par défaut NON)
HIGH_MEMORY=${HIGH_MEMORY:-NON}

# Étape 1 : Démultiplexage
cd "$PIPELINE_DIR/demul"
echo "Étape 1/4: Démultiplexage de $INDIR dans $OUTDIR"
./actions/demul-1.0.0.sh

# Étape 2 : Exécution de Vidjil
cd $PIPELINE_DIR/vidjil-v1
echo "Étape 2/4: Exécution de vidjil"

if [[ $HIGH_MEMORY = "OUI" ]]; then
    action="vidjil-high-memory-1.0.0"
else
    action="vidjil-low-memory-1.0.0"
fi

bash actions/$action.sh

# Étape 3 : Copie des fichiers fastq
cd $PIPELINE_DIR/copie
echo "Étape 3/4: Copie des fastq sur le NAS"
./actions/copie-1.0.0.sh "$OUTDIR/fastq" "/hard/coded/path/to/nas/$(basename $OUTDIR)"

# Étape 4 : Copie des résultats Vidjil
echo "Étape 4/4: Copie des fichiers vidjil sur le NAS"
./actions/copie-1.0.0.sh "$OUTDIR/vidjil-results" "/hard/coded/path/to/nas/$(basename $OUTDIR)"
```

---

## Appel direct à Snakemake dans un launcher

Il est possible d'appeler directement Snakemake dans un launcher, sans passer par une action intermédiaire. Cela simplifie la structure du pipeline et permet une intégration directe des workflows Snakemake.

### Exemple de launcher avec appel direct à Snakemake

```bash
#!/bin/bash

## HIGH_MEMORY : Variable optionnelle (OUI ou NON, par défaut NON)
HIGH_MEMORY=${HIGH_MEMORY:-NON}

SNAKEFILE=Snakefile-1.0.0

# Étape 1 : Démultiplexage
cd "$PIPELINE_DIR/demul"
echo "Étape 1/3: Démultiplexage de $INDIR dans $OUTDIR"
./actions/demul-1.0.0.sh

# Étape 2 : Exécution directe de Snakemake
cd "$PIPELINE_DIR/vidjil-v1"
echo "Étape 2/3: Exécution de Snakemake pour l'analyse Vidjil"

if [[ $HIGH_MEMORY = "OUI" ]]; then
    echo "##STEP Exécution de Snakemake avec profil haute mémoire"
    snakemake --workflow-profile high_memory -d $OUTDIR -s $SNAKEFILE
else
    echo "##STEP Exécution de Snakemake avec profil basse mémoire"
    snakemake --workflow-profile low_memory -d $OUTDIR -s $SNAKEFILE
fi

# Étape 3 : Copie des résultats
cd "$PIPELINE_DIR/copie"
echo "Étape 3/3: Copie des résultats vers $COPIE_DEST"
./actions/copie-1.0.0.sh "$OUTDIR" "$COPIE_DEST"
```

### Avantages de l'appel direct à Snakemake

- **Clarté** : Il n'est pas toujours nécessaire de passer par des actions. L'approche recommandée est d'utiliser snakemake dans le launcher, et en même temps de partager l'utilisation *via* un script (action).

---

## Variables d'environnement

Les variables suivantes sont toujours définies et **ne doivent en aucun cas** être redéfinies dans un launcher :

- **`INDIR`** : Dossier d'entrée. **Ne pas modifier.**
- **`OUTDIR`** : Dossier de sortie/travail. **Ne pas modifier.** Tous les fichiers générés doivent être écrits dans ce dossier.
- **`RUN_NAME`** : Nom de l'analyse, fourni par l'utilisateur. Peut être modifié si nécessaire (ex: suppression des espaces).
- **`PIPELINE_DIR`** : Dossier contenant tous les pipelines accessibles. Permet de rendre les pipelines portables et robustes.
- **`PIPELINE_NAME`** : Nom du pipeline auquel appartient le launcher actuel.

---

## Bonnes pratiques

1. **🏆 RÈGLE D'OR - Chemins avec `$PIPELINE_DIR`** : **Tous les chemins mentionnés dans un launcher doivent commencer par `$PIPELINE_DIR`**. Cette variable garantit la portabilité et la robustesse des pipelines en évitant les chemins codés en dur.
   
   ✅ **Correct** :
   ```bash
   cd "$PIPELINE_DIR/demul"
   ./actions/demul-1.0.0.sh
   ```
   
   ❌ **Incorrect** :
   ```bash
   cd /chemin/absolu/vers/demul
   ./actions/demul-1.0.0.sh
   ```

2. **Exécutabilité des scripts** : Toujours vérifier que les scripts sont exécutables (`chmod +x`).
3. **Gestion des erreurs** : Retourner un code non nul en cas d'échec pour arrêter le launcher.
4. **Communication avec Mercure** : Utiliser `##STEP` pour les étapes et `## ERROR` pour les erreurs.
5. **Suivi de progression** : Utiliser le format `X of 100 (X%) done` pour la progression.
6. **Exécution via SLURM** : Toujours utiliser `srun` pour exécuter les commandes.
7. **Variables d'environnement** : Respecter les variables prédéfinies et ne pas les redéfinir.
