#

## Pipeline de copie

Même s'il existe assez peu d'applications concrètes à un pipeline dédié uniquement à la copie, il est possible d'en créer un, avec un launcher dédié.
Ce launcher fera usage, pour la copie seule, de l'action associée située dans copie/actions/action-1.0.0.sh.

```
copie
├── actions
│   └── copie-1.0.0.sh
└── launchers
    └── default.sh
```

Attention, ne pas oublier de s'assurer que les scripts sont exécutables.

### Code de copie/actions/copie-1.0.0.sh

```bash
#!/bin/bash

cd "$PIPELINES_DIR/copie"

FROM=$1
TO=$2

job_name=copie-$(date +%s)

echo "SLURM run ID: $job_name"

srun --job-name="$job_name" --mem=4G rsync -a --info=progress2 "$FROM" "$TO" | tr '\r' '\n' | sed -r 's/.+\s([0-9]+)%.+/\1 of 100 (\1%) done/gm'
res=$?

if [ $res -ne 0 ]; then
    echo "##ERROR Erreur lors de la copie avec rsync (code de retour: $res)"
    exit $res
else
    echo "Copie terminée"
fi
```

Le code de cette action réprésente bien une seule étape, avec:
- la défintion des arguments du script (1 et 2, et leur variable d'environnement correspondante).
- la définition d'un nom de job SLURM (qui permet de suivre l'exécution dans Mercure)
- l'utilisation de srun: toujours exécuter une commande en passant par SLURM.

L'exemple donné illustre comment communiquer avec Mercure.

Pour informer de la progression, le code présenté ici imite la sortie de snakemake (dont Mercure sait extraire un pourcentage de complétion de la tâche).

En l'occurrence, la chaîne de caractère qui permet d'indiquer la progression est: `X of 100 (X%) done`.

Enfin, le `##ERROR`, qui permet de définir le message d'erreur pour le run dans Mercure.

Il est très important, en cas d'erreur d'une étape, de retourner avec une valeur différente de zéro.
C'est ce qui permet au launcher, qui s'exécute lui en mode strict (`set -euo pipefail`), de s'arrêter dès la première erreur.


### Code de copie/launchers/default.sh

```bash
#!/bin/bash

## COPIE_DEST La destination de copie, sous la forme d'une chaîne de caractères valable en tant que deuxième argument à rsync. Exemple: user@host:/dossier/depot

cd "$PIPELINE_DIR/copie"

echo "##STEP Etape 1/1: copie de $INDIR vers $COPIE_DEST"
./actions/copie-1.0.0.sh "$INDIR" "$COPIE_DEST"

```

Le launcher regroupe toutes les tâches à exécuter. Il est responsable de la communication de l'étape en cours avec Mercure, en affichant `##STEP ...` avec le numéro correspondant.

Copie seule: 1 launcher qui utilise 1 action.

## Pipeline de démultiplexage

```
demul
├── actions
│   └── action-1.0.0.sh
└── launchers
    ├── default.sh
    └── demul-copie.sh
```

### Code de l'action seule `demul/actions/demul-1.0.0.sh`

```bash
#!/bin/bash

demul_job_name=demul-$(date +%s)

# Permet de suivre l'évolution du run via l'interface Mercure
echo "SLURM run ID: $demul_job_name"

# Lancement du job SLURM pour le démultiplexage avec bcl-convert (en interactif de manière à bloquer le script jusqu'à la fin du job)
srun --job-name=$demul_job_name --output=$OUTDIR/demul-%j.out --mem=64G
\ singularity exec /SINGULARITIES/bcl-convert.sif bcl-convert --sample-sheet $INDIR/adn.csv
\ --output-dir $OUTDIR/fastq --bcl-input-dir $INDIR --no-lane-splitting true
```

### Code du launcher

```bash
#!/bin/bash

cd "$PIPELINE_DIR/demul"

echo "Etape 1/1: Démultiplexage de $INDIR dans $OUTDIR"

./actions/demul-1.0.0.sh 

```

## Pipeline de démultiplexage et copie

### Code du launcher `demul/launchers/demul-copie.sh`

```bash
#!/bin/bash

## COPIE_DEST La destination de copie, sous la forme d'une chaîne de caractères valable en tant que deuxième argument à rsync. Exemple: user@host:/dossier/depot

[[ -z $COPIE_DEST ]] && { echo "##ERROR COPIE_DEST est obligatoire"; exit 1; };

# Utilisation d'une première action
cd "$PIPELINE_DIR/demul"
echo "Etape 1/2: Démultiplexage de $INDIR dans $OUTDIR"
./actions/demul-1.0.0.sh 

# Utilisation d'une deuxième action
cd "$PIPELINE_DIR/copie"
echo "Etape 2/2: Copie de $OUTDIR dans $COPIE_DEST"
./actions/copie-1.0.0.sh $OUTDIR $COPIE_DEST

```

## Exemple d'une action qui utilise snakemake

Pour éviter d'avoir à écrire soi-même `echo "SLURM run ID: ..."` ou bien `echo "$X of 100 ($X%) done"`, il est possible de créer des actions qui utilisent snakemake.

Exemple:

Dans `vidjil-v1/actions/vidjil-high-memory-1.0.0.sh`

```bash
cd "$PIPELINE_DIR/vidjil-v1"
snakemake --workflow-profile high_memory -d $OUTDIR
```

et dans `vidjil-v1/actions/vidjil-low-memory-1.0.0.sh`

```bash
cd "$PIPELINE_DIR/vidjil-v1"
snakemake --workflow-profile low_memory -d $OUTDIR
```

Ainsi, dans `vidjil-v1/launchers/default.sh`, on peut envisager une sélection du profil de workflow, via une variable d'environnement optionnelle définie dans le formulaire:

```bash
#!/bin/bash

## HIGH_MEMORY Une variable qui vaut OUI ou NON (optionnelle, NON par défaut)
HIGH_MEMORY=${HIGH_MEMORY:-NON}

# Utilisation d'une première action
cd "$PIPELINE_DIR/demul"
echo "Etape 1/4: Démultiplexage de $INDIR dans $OUTDIR"
./actions/demul-1.0.0.sh 

# Utilisation de l'action principale
cd $PIPELINE_DIR/vidjil-v1

echo "Etape 2/4: Exécution de vidjil"
if [[ $HIGH_MEMORY = "OUI" ]];then
    action="vidjil-low-memory-1.0.0"
else
    action="vidjil-high-memory-1.0.0"
fi
# Exécution de l'action
bash actions/$action.sh

# Préparation de la copie
cd $PIPELINE_DIR/copie

# Copie 1
echo "Etape 3/4: copie des fastq sur le NAS"
./actions/copie-1.0.0 "$OUTDIR/fastq" "/hard/coded/path/to/nas/$(basename $OUTDIR)"

# Copie 2
echo "Etape 4/4: copie des fichiers vidjil sur le NAS"
./actions/copie-1.0.0 "$OUTDIR/vidjil-results" "/hard/coded/path/to/nas/$(basename $OUTDIR)"

```

# Important à noter

Les variables suivantes seront toujours définies, et il est interdit de les définir comme variables utilisateur dans un launcher:

- INDIR (le dossier d'entrée). La variable ne doit en aucun cas être altérée.
- OUTDIR (le dossier de sortie/de travail). La variable ne doit surtout pas être modifiée: c'est dans ce dossier, et dans ce dossier uniquement, que le pipeline doit écrire.
- RUN_NAME (le nom de l'analyse, donné par l'utilisateur). Celui-ci peut être altéré (notamment s'il est nécessaire d'en retirer les espaces)
- PIPELINE_DIR: c'est le dossier où se trouvent tous les pipelines auxquels l'utilisateur peut faire référence. L'objectif de cette variable est de rendre les pipelines portables et robustes.
- PIPELINE_NAME: le nom du pipeline auquel appartient le launcher actuel.