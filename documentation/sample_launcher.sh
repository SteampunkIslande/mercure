#!/bin/bash

# Ci-dessous, les variables d'environnement à définir avant de lancer le script
# Formalisme: ## VARIABLE Description

## BED Nom du fichier bed (sans l'extension .bed)
## PANEL_NAME Nom du panel d'analyse (correspond au nom du fichier de config .yaml)
## MOABI_DIR Répertoire de destination sur le stockage MOABI (forme: utilisateur@adresse:/chemin/vers/dossier)

# Les variables suivantes sont exportées par le script de soumission du run (généré par Mercure):

# PIPELINE_DIR est exporté par le script de soumission du run
# PIPELINE_NAME est exporté par le script de soumission du run
# INDIR est exporté par le script de soumission du run
# OUTDIR est exporté par le script de soumission du run
# RUN_NAME est exporté par le script de soumission du run

cd $PIPELINE_DIR/$PIPELINE_NAME

echo "## STEP Etape 1/4: Démultiplexage"

demul_job_name=$(date +%s)

# Permet de suivre l'évolution du run via l'interface Mercure
echo "SLURM run ID: $demul_job_name"

# Lancement du job SLURM pour le démultiplexage avec bcl-convert (en interactif de manière à bloquer le script jusqu'à la fin du job)
srun --job-name=$demul_job_name --output=$OUTDIR/slurm%j.out --mem=64G \
    singularity exec /SINGULARITIES/bcl-convert.sif bcl-convert --sample-sheet $INDIR/adn.csv \
    --output-dir $OUTDIR/fastq --bcl-input-dir $INDIR --no-lane-splitting true

echo "## STEP Etape 2/4: Copie des fastq sur MOABI"

# Copie des fastq générés vers le stockage MOABI (même nom que le dossier d'analyse local)
rsync -a $OUTDIR/fastq $MOABI_DIR/$(basename $OUTDIR)

echo "## STEP Etape 3/4: Analyse avec smaug"

# Lancement de l'analyse smaug (via snakemake)
snakemake --config indir=$INDIR outdir=$OUTDIR bed=/reference/bed/$PANEL_NAME/$BED.bed --configfile configs/$PANEL_NAME.yaml

# Copie des résultats sur le NAS local
echo "## STEP Etape 4/4: Copie des résultats sur le NAS local"
rsync -a $OUTDIR/bam /mnt/genetique/$RUN_NAME