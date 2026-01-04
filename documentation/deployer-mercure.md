# Installation

## Préambule: dossiers de travail

- Dossier `/OPT/mercure`. Stockage des images singularity, configuration de l'application web, tous les scripts, dossier uploads, base de données (`mercure.db`).
- Dossier des pipelines (production): `/OPT/pipelines`
- Dossiers d'exécution des scripts: `/OPT/JOBS/{TODO,RUNNING,DONE,FAILS,LOGS}`. Ces dossiers doivent absolument être vides à chaque changement de la base de données.

## Obtenir les images singularity correpsondantes

Exécuter `./mercure-build.sh` (dans ce dépôt). Deux fichiers seront générés: `mercure-routine.sif` et `mercure-webapp.sif`. (requiert singularity)

Placer ces fichiers `.sif` dans le dossier `/OPT/mercure`, en s'assurant qu'ils appartiennent bien à l'utilisateur `hermes`.

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
