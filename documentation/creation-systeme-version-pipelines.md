# Présentation du système de versionnement des pipelines

L'objectif est de distinguer le dossier de développement des pipelines, qui doit servir de zone d'attente avant déploiement, du dossier des pipelines en production.

A cette fin, il existe trois dossiers, chacun remplissant une fonction dans le déploiement continu des pipelines, sans que l'exécution de la production ne puisse être impactée.

## Dossier `/OPT/pipelines-dev`

Ici, tout utilisateur appartenant au groupe `bioinfo` a le droit de modifier tous les fichiers qu'il souhaite pour déployer son pipeline. Dans ce dossier (versionné à la racine, sans submodule), chaque sous-dossier est un pipeline. Lorsqu'un utilisateur considère que son pipeline est prêt à être déployé, il peut tenter un `git commit`. Un hook de pre-commit va alors s'exécuter, et rejeter le commit si ce dernier contrevient aux règles de déploiement d'un pipeline. Les règles sont les suivantes:

- `$PIPELINE_NAME/launchers/*.sh`: tout fichier .sh situé dans `launchers` peut avoir autant de commits associés que souhaité.
- `$PIPELINE_NAME/actions/*.sh`: tout fichier .sh situé dans `actions` doit avoir une (et une seule) révision associée. La raison est que dès lors qu'une action a été validée (qu'elle a un commit associé), on considère que tout autre pipeline dans ce dépôt a pu l'utiliser et peut donc encore en dépendre pour son fonctionnement. La modifier ou la supprimer est donc hors de question: une action peut être marquée comme obsolète, mais dans ce cas, il revient au développeur du pipeline qui utilise de telles actions d'y rémédier. Le nom de ces fichiers `.sh` doit suivre SemVer (Majeure.Mineure.Patch).
- Chaque fichier `.sh` situé dans `actions` doit avoir un fichier `.md` associé correspondant à une documentation. Ce fichier peut être modifié, contrairement à son action associée. Ce fichier a une taille maximale de 100Ko.
- Tous les autres fichiers doivent être versionnés selon SemVer (Majeure.Mineure.Patch) et ne peuvent avoir qu'une seule et unique révision (au moment de leur création).

Dès lors que l'utilisateur a modifié son pipeline et souhaite le voir déployé en production, il doit exécuter les trois commandes suivantes:

- `git add $PIPELINE_NAME/launcher.sh ...`: ajout des modifications effectuées à la validation. Attention, si vous exécutez `git add .` et que d'autres bioinfos ont modifié un de leurs pipelines, sera ajouté à la validation

## Dossier `/OPT/pipelines`

Il s'agit du dossier contenant les pipelines en production. La routine s'assure qu'en dehors des vérifications de mises à jours disponibles (toutes les deux minutes environ), ce dossier est read-only. Cela permet d'éviter toute modification accidentelle de ce dossier.
Toutes les deux minutes, donc, la routine exécute `git pull` depuis le dossier `/OPT/pipelines`, tel que configuré dans le `Rocket.toml`. Si de nouveaux commits sont disponibles sur le dépôt distant (dans `/OPT/pipelines.git`), ils seront appliqués à `/OPT/pipelines`.
Afin de s'assurer que la mise à jour du dossier `/OPT/pipelines` n'impacte pas la reproductibilité des pipelines, deux mécanismes sont en place, présentés dans les sections suivantes:

- Mise à jour uniquement lorsqu'aucun run n'est dans le statut `Pending` (affichage: `En attente`).
- Le dossier `/OPT/pipelines-dev` possède un "git hook" de `pre-commit` qui vérifie la conformité des commits.

### Mises à jour

La routine s'exécute périodiquement, en deux phases: la première consiste à traiter les runs marqués comme `Pending`: si le type de dossier d'entrée est `Run Brut Illumina`, alors la routine fait appel à un script, défini dans `Rocket.toml` *via* l'option `check_run_completed` (qui indique le chemin absolu du script de test de la fin des runs Illumina). Si le run est terminé, la routine passe la tentative (et le run associé) au statut `Running`. Sinon, les runs des autres types (`Dossier ONT`, ou `Dossier d'analyse`) en statut `Pending` sont immédiatement passés au statut `Running`. Dans tous les cas, le passage de `Pending` à `Running` déclenche la création d'un script, où le contenu du launcher indiqué dans le formulaire est copié tout entier dans un script shell dans `/OPT/JOBS/TODO/job-{numéro de run}-{numéro de tentative}.sh`. Ainsi, même si le launcher du job qui vient d'être ajouté à `/OPT/JOBS/TODO` a été modifié, c'est la version précédente qui sera exécutée et il n'y aura pas de conflit.

### Conformité des commits

Un hook git de pre-commit permet de s'assurer que le commit respecte bien les préconisations suivantes:

- `$PIPELINE_NAME/launchers/*.sh`: tout fichier .sh situé dans `launchers` peut avoir autant de commits associés que souhaité.
- `$PIPELINE_NAME/actions/*.sh`: tout fichier .sh situé dans `actions` doit avoir une (et une seule) révision associée. La raison est que dès lors qu'une action a été validée (qu'elle a un commit associé), on considère que tout autre pipeline dans ce dépôt a pu l'utiliser et peut donc encore en dépendre pour son fonctionnement. La modifier ou la supprimer est donc hors de question: une action peut être marquée comme obsolète, mais dans ce cas, il revient au développeur du pipeline qui utilise de telles actions d'y rémédier. Le nom de ces fichiers `.sh` doit suivre SemVer (Majeure.Mineure.Patch).
- Chaque fichier `.sh` situé dans `actions` doit avoir un fichier `.md` associé correspondant à une documentation. Ce fichier peut être modifié, contrairement à son action associée. Ce fichier a une taille maximale de 100Ko.
- Tous les autres fichiers doivent être versionnés selon SemVer (Majeure.Mineure.Patch) et ne peuvent avoir qu'une seule et unique révision (au moment de leur création).


## Dossier `/OPT/pipelines.git`

Il s'agit d'un dépôt git "bare", qui sert d'origine pour les dépôts `/OPT/pipelines-dev` et `/OPT/pipelines`.

Ce dernier est possédé par l'utilisateur `hermes`, et en mode shared=group.

Attention: ce dossier doit être monté 

