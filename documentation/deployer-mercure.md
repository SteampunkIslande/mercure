# Guide de Déploiement Mercure

Ce guide décrit les étapes nécessaires pour déployer Mercure en production ou en test.

## Vue d'ensemble

Mercure nécessite trois répertoires principaux :

- **MERCURE_DIR** : Contient les images Singularity, fichiers de configuration et scripts
- **JOBS_DIR** : Dossier lié à la base de données avec la structure `{TODO,RUNNING,DONE,FAILS,LOGS}`
- **PIPELINES_DIR** : Dossier où Mercure trouve les pipelines

## Prérequis

- Utilisateur `hermes` configuré
- Singularity installé
- Accès aux répertoires de destination
- Permissions d'écriture sur les dossiers de travail

## Structure des Répertoires

### Production
```
/OPT/mercure/           # MERCURE_DIR - Images SIF, config, scripts, uploads, mercure.db
/OPT/pipelines/         # PIPELINES_DIR - Pipelines de production  
/OPT/JOBS/              # JOBS_DIR - {TODO,RUNNING,DONE,FAILS,LOGS}
```

### Test
```
/OPT/mercure-test/      # MERCURE_DIR - Images SIF, config, scripts, uploads, mercure.db
/OPT/pipelines-test/    # PIPELINES_DIR - Pipelines de test
/OPT/JOBS-TEST/         # JOBS_DIR - {TODO,RUNNING,DONE,FAILS,LOGS}
```

⚠️ **Important** : Les dossiers JOBS doivent être vidés à chaque changement de base de données.

## Étapes d'Installation

### 1. Préparer l'environnement

Créer la structure de répertoires (en tant qu'utilisateur hermes):
```bash
# Pour la production
mkdir -p /OPT/mercure/uploads
mkdir -p /OPT/pipelines
mkdir -p /OPT/JOBS/{TODO,RUNNING,DONE,FAILS,LOGS}

# Pour les tests  
mkdir -p /OPT/mercure-test/uploads
mkdir -p /OPT/pipelines-test
mkdir -p /OPT/JOBS-TEST/{TODO,RUNNING,DONE,FAILS,LOGS}

# Permissions
chown -R hermes:hermes /OPT/mercure* /OPT/JOBS* /OPT/pipelines*
```

### 2. Générer les images Singularity

```bash
# Depuis le répertoire du projet
./mercure-build.sh
```

Cela génère :
- [`mercure-routine.sif`](mercure-routine.def:1)
- [`mercure-webapp.sif`](mercure-webapp.def:1)

Copier ces fichiers dans `MERCURE_DIR` :
```bash
cp mercure-*.sif /OPT/mercure/  # ou /OPT/mercure-test/
chown hermes:hermes /OPT/mercure/mercure-*.sif
```

### 3. Copier les fichiers de configuration

Copier les fichiers nécessaires dans `MERCURE_DIR` :

```bash
# Scripts essentiels
cp scripts/mercure-run.sh /OPT/mercure/
cp scripts/cronjob.sh /OPT/mercure/
cp scripts/run-completed.sh /OPT/mercure/
cp scripts/post-run-script.sh /OPT/mercure/

# Configuration
cp Rocket.toml /OPT/mercure/

# Permissions
chown hermes:hermes /OPT/mercure/*
chmod +x /OPT/mercure/*.sh
```

### 4. Initialiser la base de données

```bash
cd /OPT/mercure
touch mercure.db
chown hermes:hermes mercure.db
```

### 5. Configurer les fichiers

#### [`mercure-run.sh`](scripts/mercure-run.sh:1)
Vérifier/modifier les variables d'environnement :
```bash
MERCURE_DIR="/OPT/mercure"          # ou /OPT/mercure-test
JOBS_DIR="/OPT/JOBS"                # ou /OPT/JOBS-TEST  
PIPELINES_DIR="/OPT/pipelines"      # ou /OPT/pipelines-test
```

#### [`Rocket.toml`](Rocket.toml:1)
Configurer la section `[release]` pour la production :

```toml
[release]
address = "0.0.0.0"  # IP d'écoute
mercure_db = "sqlite:///OPT/mercure/mercure.db"
template_dir = "/static/templates"
static_dir = "/static" 
upload_dir = "/OPT/mercure/uploads"
logs_dir = "/OPT/JOBS/LOGS"
pipeline_dir = "/OPT/pipelines"
jobs_dir = "/OPT/JOBS"
sequencers_dir = "/data/raw/sequenceurs"
analysis_dir = "/data/analysis"
ont_dir = "/data/raw/sequenceurs/GRIDION/output"
check_run_completed = "/OPT/mercure/run-completed.sh"
```

### 6. Configurer le cron

Éditer le [`cronjob.sh`](scripts/cronjob.sh:1) avec les bons chemins :
```bash
JOBS_DIR=/OPT/JOBS  # Adapter selon l'environnement
```

Ajouter au crontab de l'utilisateur `hermes` :
```bash
sudo -u hermes crontab -e
# Ajouter : * * * * * /OPT/mercure/cronjob.sh
```

## Démarrage

Lancer Mercure en tant qu'utilisateur `hermes` :

```bash
sudo -u hermes /OPT/mercure/mercure-run.sh
```

Cette commande démarre deux instances Singularity :
- `mercure-webappd` : Application web
- `mercure-routined` : Routine de traitement

## Vérification

Vérifier que les instances sont actives :
```bash
singularity instance list
```

L'application web sera accessible sur l'IP et le port configurés dans [`Rocket.toml`](Rocket.toml:1).

## Scripts de Fonctionnement

### [`run-completed.sh`](scripts/run-completed.sh:1)
- Prend en paramètres : chemin du dossier de séquençage, nom du séquenceur
- Retourne 0 si le run est terminé, 1 sinon
- Vérifie la présence de `CopyComplete.txt` ou `RTAComplete.txt` + `CompletedJobInfo.xml`

### [`post-run-script.sh`](scripts/post-run-script.sh:1)  
- Exécuté après chaque analyse
- Extrait les statistiques SLURM des jobs
- Génère des fichiers de suivi des ressources

### [`cronjob.sh`](scripts/cronjob.sh:1)
- Traite les jobs en attente dans `JOBS_DIR/TODO`
- Déplace les jobs selon leur statut (RUNNING → DONE/FAILS)
- Génère les logs de reproductibilité

## Dépannage

- **Vérifier les permissions** : tous les fichiers doivent appartenir à `hermes`
- **Vérifier les chemins** : s'assurer que tous les chemins dans la configuration sont corrects
- **Logs** : consulter les logs dans `JOBS_DIR/LOGS`
- **Instances Singularity** : utiliser [`singularity instance list`] pour vérifier l'état
