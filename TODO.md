# Ré-organisation des fichiers

## Routes

- [x] Déplacer back_end/redirect.rs dans frontend (et renommer la route correspondante de post en get)

## Scripts

- [ ] Le dossier scripts ne devrait contenir que des scripts tels qu'ils sont en production. Pour les mettre à jour: copier depuis l'installation (fonctionnelle) présente sur le serveur de production.

## Utilitaires

Nettoyage de `utils.rs`.

- [ ] Créer un fichier dédié pour les samplesheets Illumina.
- [ ] déplacer la fonction parse_launcher de `utils.rs` vers `launchers_check.rs`.

# Remaniement de code

## Langue

- [ ] S'assurer que tous les commentaires de code sont en anglais, mais que toutes les chaînes de caractères en lien avec les utilisateurs (affichage des erreurs notamment) soient en français. Pour les valeurs d'enum, celles-ci doivent être en anglais, mais traduites par une simple table de traduction en jinja dans le template.

## Rafraîchissements

- [ ] Revoir la route `/home` (front_end/home.rs)
- [ ] Remanier le modèle HgRun pour toujours utiliser les champs indir et outdir (même pour les dossiers BCL).