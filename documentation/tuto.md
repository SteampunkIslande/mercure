# Développer un pipeline

Pour développer un pipeline utilisable dans Mercure, vous devez travailler dans un dossier situé dans votre `$HOME`.

## Architecture

Un dossier de pipeline doit comporter au moins un sous-dossier `launchers`, dans lequel chaque fichier `.sh` sera considéré comme un launcher à part entière.

## Contenu d'un fichier launcher

Le fichier doit être exécutable par tous

Le script doit obligatoirement comporter ces deux lignes:
```bash
#!/bin/bash
cd "$PIPELINE_DIR/$PIPELINE_NAME"
```

Le `#!`, pour rendre le launcher exécutable.
Le dossier de travail doit obligatoirement être celui du pipeline (avec le(s) launcher, le(s) snakefile, l'ensemble des fichiers de config, et d'éventuels scripts pour aider à l'exécution).

Utiliser les variables d'environnement `$PIPELINE_DIR` et `$PIPELINE_NAME` permet de rendre le script launcher portable.

Le fichier launcher peut ré-utiliser des scripts d'autres pipelines, en passant par des `actions`, qui sont des scripts que les bio-infos peuvent mettre à disposition de tous.

Une action (située dans le dossiers actions du pipeline), est un script, obligatoirement versionné par SemVer (Majeur.Mineur.Patch), et dont le seul nombre de révisions acceptables dans git est un. En effet, dès lors qu'une action a été créée et testée avec succès, on considère que n'importe quel utilisateur peut l'avoir utilisée une fois dans un de ses pipelines.

# Déclarer un pipeline dans Mercure




