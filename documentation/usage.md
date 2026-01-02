# Structure attendue d'un dossier de pipeline

Dans un souci de reproductibilité, le dossier entier où sont stockés les pipelines est versionné par git.

Pour un pipeline donné, il est nécessaire de créer au moins un des deux dossiers `launchers` et `actions`, dont les fonctions respectives sont décrites dans les paragraphes suivants.

## Dossier launchers

Dans le dossier `launchers`, on trouve au moins un fichier `.sh`, dont le rôle est l'exécution d'un pipeline bioinfo de bout en bout.

Le rôle de ce script est non seulement l'appel des différents composants du pipeline, mais également la communication avec Mercure à travers l'écriture, dans stdout, de messages à destination de l'utilisateur.

Il existe pour cela trois types de messages:

- `## STEP Etape X/Y: Description de l'étape`: Mercure affichera la dernière apparition de ce message dans le log d'exécution
- `SLURM run ID: nom_du_job_slurm`: Mercure listera tous les jobs en queue avec ce nom. Snakemake fournit déjà cette information pour l'ensemble du pipeline.
- `\d+ of \d+ steps (\d+%) done`: Mercure affichera une barre de progression avec la valeur du pourcentage. Snakemake fournit aussi cette information au cours de l'exécution du pipeline.

## Dossier actions

Dans le dossier `actions`, on trouve des fichiers `.sh`, qui permettent la réutilisation du code des pipelines.

Chaque script doit être documenté (avec un .md du même nom), et son nom doit refléter sa version (majeure.mineure.révision).
Le nombre de révisions de ce script ne peut être que de 1 exactement: il peut être créé, mais jamais supprimé ni renommé. Cela garantit à tous les utilisateurs que le comportement de ces scripts est constant.
