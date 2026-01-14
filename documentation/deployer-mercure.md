# Installation


Trois dossiers importants:
- MERCURE_DIR: où sont les SIF de mercure, ses fichiers de conf, et ses scripts
- JOBS_DIR: un dossier, lié de façon unique à une base de donnée. `mkdir -p $JOBS_DIR/{TODO,RUNNING,DONE,FAILS,LOGS}`
- PIPELINES_DIR: le dossier où mercure va trouver les pipelines

## Préambule: dossiers de travail

- Dossier `/OPT/mercure`. Stockage des images singularity, configuration de l'application web, tous les scripts, dossier uploads, base de données (`mercure.db`).
- Dossier des pipelines (production): `/OPT/pipelines`
- Dossiers d'exécution des scripts: `/OPT/JOBS/{TODO,RUNNING,DONE,FAILS,LOGS}`. Ces dossiers doivent absolument être vides à chaque changement de la base de données.

Pour les tests:
- Dossier `/OPT/mercure-test`. Stockage des images singularity, configuration de l'application web, tous les scripts, dossier uploads, base de données (`mercure.db`).
- Dossier des pipelines (production): `/OPT/pipelines-test`
- Dossiers d'exécution des scripts: `/OPT/JOBS-TEST/{TODO,RUNNING,DONE,FAILS,LOGS}`. Ces dossiers doivent absolument être vides à chaque changement de la base de données.

## Présentation des fichiers nécessaires

Dans le dossier de travail de mercure (`/OPT/mercure` en production ou `/OPT/mercure-test` en développement), il faut s'assurer, avant de commencer, que les dossiers et fichiers suivants sont bien présents:

- Dossier uploads (et renseigner sa valeur dans Rocket.toml)
- cronjob.sh (et ajouter une entrée dans le crontab)
- mercure.db: l'instance a besoin que le fichier existe avant le lancement: touch mercure.db. Ne pas oublier de renseigner le chemin de la base de données dans Rocket.toml
- mercure-routine.sif: l'image générée par mercure-build.sh
- mercure-webapp.sif: l'image générée par mercure-build.sh
- mercure-run.sh: s'assurer que la variable d'environnement MERCURE_DIR indique bien le dossier de travail considéré (par défaut, c'est le chemin de la production qui est indiqué).
- post-run-script.sh: renseigner son chemin absolu dans Rocket.toml. Ce dernier sera exécuté par la routine

Edition des fichiers (paramétrage)



## Obtenir les images singularity correspondantes

Exécuter `./mercure-build.sh` (dans ce dépôt). Deux fichiers seront générés: `mercure-routine.sif` et `mercure-webapp.sif`. (requiert singularity)

Placer ces fichiers `.sif` dans le dossier `/OPT/mercure` (ou `/OPT/mercure-test`), en s'assurant qu'ils appartiennent bien à l'utilisateur `hermes`.


## Scripts de fonctionnement

Les scripts suivants doivent être situés dans `/OPT/mercure`

- `run-completed.sh`: le chemin absolu de ce script doit être indiqué dans `Rocket.toml`. Ce script prend deux arguments: Le chemin absolu du dossier de séquençage et le nom du séquenceur. Doit renvoyer vrai (0) si le run est terminé, et 1 sinon. Le premier argument (le nom du dossier de run) existe nécessairement (la routine de `mercure` n'appelle pas ce script si le dossier n'existe pas).
- `post-run-script.sh`: le chemin absolu de ce script doit être indiqué dans `Rocket.toml`. Ce script est appelé

## Configuration

- `Rocket.toml`: Doit se situer dans `/OPT/mercure`

# Exécution

Lancer, en tant qu'utilsateur `hermes`, `/OPT/mercure/mercure-run.sh`. Cela créera deux instances singularity (`mercure-webappd` et `mercure-routined`, respectivement l'application web, et la routine).

## Application web


# Administration
