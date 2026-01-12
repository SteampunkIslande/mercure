# Présentation du système de versionnement des pipelines

L'objectif est de distinguer le dossier de développement des pipelines, qui doit servir de zone d'attente avant déploiement, du dossier des pipelines en production.

A cette fin, il existe trois dossiers, chacun remplissant une fonction dans le déploiement continu des pipelines, sans que l'exécution de la production ne puisse être impactée.

## Dossier `/OPT/pipelines-dev`

Ici, tout utilisateur appartenant au groupe `bioinfo` a le droit de modifier tous les fichiers qu'il souhaite pour déployer son pipeline. Dans ce dossier (versionné à la racine, sans submodule), chaque sous-dossier est un pipeline. Lorsqu'un utilisateur considère que son pipeline est prêt à être déployé, il peut tenter un `git commit`. Un hook de pre-commit va alors s'exécuter, et rejeter le commit si ce dernier contrevient aux règles de déploiement d'un pipeline. Les règles sont les suivantes:

- `$PIPELINE_NAME/launchers/*.sh`: tout fichier .sh situé dans `launchers` peut avoir autant de commits associés que souhaité.
- `$PIPELINE_NAME/actions/*.sh`: tout fichier .sh situé dans `actions` doit avoir une (et une seule) révision associée. La raison est que dès lors qu'une action a été validée (qu'elle a un commit associé), on considère que tout autre pipeline dans ce dépôt a pu l'utiliser et peut donc encore en dépendre pour son fonctionnement. La modifier ou la supprimer est donc hors de question: une action peut être marquée comme obsolète, mais dans ce cas, il revient au développeur du pipeline qui utilise de telles actions d'y rémédier. Le nom de ces fichiers `.sh` doit suivre SemVer (Majeure.Mineure.Patch).
- Chaque fichier `.sh` situé dans `actions` doit avoir un fichier `.md` associé correspondant à une documentation. Ce fichier peut être modifié, contrairement à son action associée. Ce fichier a une taille maximale de 100Ko.
- Tous les autres fichiers doivent être versionnés selon SemVer (Majeure.Mineure.Patch) et ne peuvent avoir qu'une seule et unique révision.
