# Remaniement de code

## Langue

- [X] S'assurer que tous les commentaires de code sont en anglais, mais que toutes les chaînes de caractères en lien avec les utilisateurs (affichage des erreurs notamment) soient en français. Pour les valeurs d'enum, celles-ci doivent être en anglais, mais traduites par une simple table de traduction en jinja dans le template.

## Rafraîchissements

- [X] Ajouter une route /smaug/data qui sert statiquement le sous-dossier `exports-{{ attempt_number }}` du dossier d'analyse (avec une profondeur jusqu'à 1, c'est-à-dire que l'exécution d'un script peut venir écrire des sous-dossiers, par exemple `exports-1/logs/fichier.log`, ou `exports-1/rapports/rapport.pdf`).

- [X] Faire en sorte de rendre plus cohérente la gestion des erreurs dans la routine: problème de la responsabilité de la mise à jour de la base de données. Actuellement, c'est `run_script` qui écrit dans la base de données, mais quand `run_script` renvoie une erreur, personne n'écrit dans la base (il y a juste un `error!(e)` qui est loggé...). Objectif: c'est l'appelant de `run_script` qui a la responsabilité de gérer les erreurs. `run_script` doit, pour cela, renvoyer une `anyhow::Error` qui doit alors être `downcast_ref`, pour traiter les différents cas (ex: en cas de `GitCheckError` ou `io::Result` ou toute autre erreur survenant avant même le lancement de l'analyse à proprement parler, appeler `failure_cannot_analyse_run`).

- [X] Ajouter un avertissement quand, au moment d'éditer un run, le formulaire n'a pas les mêmes variables qu'à l'exécution précédente (comparer la liste des clés dans la colonne `user_defined_vars` de la table `Run` avec la liste des noms de variables dans le formulaire **à jour** correspondant). Le message doit être explicite, un simple avertissement, qui dit: `Le formulaire 'pipeline_name/.forms/version_simple.yaml' sur la branche 'main' n'a pas les mêmes variables qu'à l'exécution précédente`.

- [X] Faire que l'erreur soit plus claire lorsque l'utilisateur veut éditer un run dont le formulaire `(nom de la branche, nom du formulaire)` n'existe plus.

- [X] Les erreurs Frontend devraient être plus claires: le `downcast_ref` devrait prendre en charge plus de cas.

- [X] Ajouter une route `smaug/uploads` qui sert statiquement les fichiers de l'utilisateur.

## Intégration continue

- [X] Ajouter au fichier `.github/workflows/release.yml` de lire la version dans `Cargo.toml`. La version est lue dans `Cargo.toml` (étape `Read version from Cargo.toml`) et utilisée pour le tag par défaut (`v<version>`), sauf si un `tag_name` est fourni manuellement. Un script `scripts/release.sh` permet de bump la version (major/minor/patch), de maj `Cargo.toml` + `Cargo.lock`, de commit/tag/pusher, et de déclencher la release.