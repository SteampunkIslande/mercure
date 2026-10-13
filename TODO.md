# Ré-organisation des fichiers

## Routes

- [x] Déplacer back_end/redirect.rs dans frontend (et renommer la route correspondante de post en get)

# Remaniement de code

## Langue

- [ ] S'assurer que tous les commentaires de code sont en anglais, mais que toutes les chaînes de caractères en lien avec les utilisateurs (affichage des erreurs notamment) soient en français. Pour les valeurs d'enum, celles-ci doivent être en anglais, mais traduites par une simple table de traduction en jinja dans le template.

## Rafraîchissements

- [ ] Revoir la route `/home` (front_end/home.rs)
- [ ] Revoir `front_end/mod.rs`, la gestion des erreurs n'est pas élégante...