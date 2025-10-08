# Documentation des Routes de l'Application Web Mercure

Cette application web Rust utilise le framework Rocket avec des routes séparées entre backend (API) et frontend (rendu de templates).

## Routes Backend (API)

Les routes backend utilisent principalement des méthodes POST pour les opérations de données et retournent du JSON.

### Authentification

#### `/mercure/api/login` (POST)
- **Méthode**: [`login_post()`](src/routes/back_end/login.rs:18)
- **Schéma JSON**:
```json
{
  "usermail": "string",
  "password": "string"
}
```
- **Description**: Authentifie un utilisateur et crée une session
- **Retour**: Redirection vers `/mercure/admin/dashboard` (admin) ou `/mercure/home` (utilisateur)

#### `/mercure/api/logout` (GET)
- **Méthode**: [`logout_get()`](src/routes/back_end/logout.rs:6)
- **Description**: Déconnecte l'utilisateur et supprime la session
- **Retour**: Redirection vers `/mercure`

### Gestion des utilisateurs

#### `/mercure/api/register` (POST)
- **Méthode**: [`register_post()`](src/routes/back_end/newuser.rs:12)
- **Authentification**: Admin requis
- **Schéma JSON**:
```json
{
  "usermail": "string",
  "username": "string",
  "password": "string"
}
```
- **Description**: Création d'un nouvel utilisateur (admin uniquement)

#### `/mercure/api/users/list` (GET)
- **Méthode**: [`list_users()`](src/routes/back_end/listusers.rs:6)
- **Description**: Liste tous les utilisateurs
- **Retour JSON**:
```json
{
  "success": true,
  "data": [
    {
      "id": 1,
      "usermail": "string",
      "username": "string",
      "created_at": "datetime",
      "last_login": "datetime|null",
      "is_admin": false
    }
  ]
}
```

#### `/mercure/api/passedit` (POST)
- **Méthode**: [`password_edit_post()`](src/routes/back_end/passedit.rs:8)
- **Schéma JSON**:
```json
{
  "user_id": 1,
  "old_password": "string|null",
  "new_password": "string"
}
```
- **Description**: Modification de mot de passe (utilisateur ou admin)

### Gestion des groupes

#### `/mercure/api/groups/list` (GET)
- **Méthode**: [`list_groups()`](src/routes/back_end/listgroups.rs:6)
- **Description**: Liste tous les groupes
- **Retour JSON**:
```json
{
  "success": true,
  "data": [
    {
      "id": 1,
      "name": "string"
    }
  ]
}
```

#### `/mercure/api/groups/list/<user_id>` (GET)
- **Méthode**: [`list_groups_for_user()`](src/routes/back_end/listgroups.rs:14)
- **Description**: Liste les groupes d'un utilisateur spécifique

#### `/mercure/api/newgroup/<group_name>` (GET)
- **Méthode**: [`newgroup_get()`](src/routes/back_end/newgroup.rs:10)
- **Authentification**: Admin requis
- **Description**: Création d'un nouveau groupe
- **Retour JSON**:
```json
{
  "success": true,
  "data": {
    "group_id": 1,
    "group_name": "string"
  }
}
```

#### `/mercure/api/groups/update` (POST)
- **Méthode**: [`update_groups()`](src/routes/back_end/editgroups.rs:14)
- **Authentification**: Admin requis
- **Schéma JSON**:
```json
{
  "to_remove": [
    {
      "id": 1,
      "name": "string"
    }
  ],
  "to_add": [
    {
      "id": 2,
      "name": "string"
    }
  ],
  "user_id": 1
}
```
- **Description**: Mise à jour des groupes d'un utilisateur

### Gestion des formulaires

#### `/mercure/api/forms/all` (GET)
- **Méthode**: [`get_all_forms()`](src/routes/back_end/getform.rs:10)
- **Description**: Récupère tous les formulaires
- **Retour JSON**:
```json
{
  "success": true,
  "data": [
    {
      "formid": 1,
      "enabled": true,
      "form_name": "string",
      "version": 1
    }
  ]
}
```

#### `/mercure/api/forms/groups/<group_id>` (GET)
- **Méthode**: [`get_all_forms_for_group()`](src/routes/back_end/getform.rs:18)
- **Description**: Récupère les formulaires associés à un groupe
- **Retour JSON**:
```json
{
  "success": true,
  "data": {
    "with_group": [...],
    "without_group": [...]
  }
}
```

#### `/mercure/api/forms/<id>` (GET)
- **Méthode**: [`get_form_from_id()`](src/routes/back_end/getform.rs:32)
- **Description**: Récupère un formulaire par son ID
- **Retour JSON**:
```json
{
  "success": true,
  "data": {
    "pipeline_name": "string",
    "launcher_name": "string",
    "form_name": "string",
    "enabled": true,
    "version": 1,
    "groups": [...],
    "user_defined_vars": {
      "variable_name": {
        "FromValuesList": {
          "allowed": ["value1", "value2"]
        }
      }
    }
  }
}
```

#### `/mercure/api/newform` (POST)
- **Méthode**: [`newform_post()`](src/routes/back_end/newform.rs:52)
- **Authentification**: Admin requis
- **Schéma JSON**: Même structure que le retour de `/mercure/api/forms/<id>`
- **Description**: Création d'un nouveau formulaire

#### `/mercure/api/enable/<formid>` (GET)
- **Méthode**: [`enable_form()`](src/routes/back_end/newform.rs:32)
- **Authentification**: Admin requis
- **Description**: Active un formulaire

#### `/mercure/api/disable/<formid>` (GET)
- **Méthode**: [`disable_form()`](src/routes/back_end/newform.rs:11)
- **Authentification**: Admin requis
- **Description**: Désactive un formulaire

### Gestion des runs

#### `/mercure/api/newrun` (POST)
- **Méthode**: [`newrun_post()`](src/routes/back_end/submitrun.rs:13)
- **Schéma JSON**:
```json
{
  "form_id": 1,
  "user_id": 1,
  "run_name": "string",
  "run_date": "string",
  "run_sequencer": "string",
  "run_flowcellid": "string",
  "sample_sheet_adn_path": "string",
  "sample_sheet_arn_path": "string",
  "metadata_path": "string",
  "user_defined_vars": {
    "key": "value"
  }
}
```
- **Description**: Création d'un nouveau run
- **Retour JSON**:
```json
{
  "success": true,
  "data": {
    "message": "Run créé avec succès!",
    "run_id": 1
  }
}
```

### Upload de fichiers

#### `/mercure/api/upload` (POST)
- **Méthode**: [`upload_post()`](src/routes/back_end/upload.rs:15)
- **Format**: Multipart form data
- **Champs**:
  - `file`: Fichier à uploader
  - `file_name_base`: Nom de base du fichier
- **Description**: Upload d'un fichier vers le serveur
- **Retour JSON**:
```json
{
  "success": true,
  "data": "/path/to/uploaded/file"
}
```

## Routes Frontend (Rendu de templates)

Les routes frontend utilisent principalement des méthodes GET et retournent du HTML via des templates Jinja2.

### Pages génériques

#### `/mercure` (GET)
- **Méthode**: [`welcome_page_get()`](src/routes/front_end/generics.rs:6)
- **Template**: [`common/welcome.html.j2`](static/templates/common/welcome.html.j2)
- **Description**: Page d'accueil de l'application

#### `/mercure/login` (GET)
- **Méthode**: [`login_get()`](src/routes/front_end/generics.rs:11)
- **Template**: [`common/login.html.j2`](static/templates/common/login.html.j2)
- **Description**: Page de connexion

#### `/mercure/success` (GET)
- **Méthode**: [`success_page_get()`](src/routes/front_end/generics.rs:16)
- **Template**: [`common/success.html.j2`](static/templates/common/success.html.j2)
- **Paramètres**: `origin`, `message`
- **Description**: Page de confirmation d'action

#### `/mercure/home` (GET)
- **Méthode**: [`home_get()`](src/routes/front_end/home.rs:6)
- **Template**: [`common/home.html.j2`](static/templates/common/home.html.j2)
- **Authentification**: Utilisateur connecté
- **Description**: Page d'accueil utilisateur avec liste des groupes

### Administration

#### `/mercure/admin/dashboard` (GET)
- **Méthode**: [`admin_dashboard_get()`](src/routes/front_end/admin.rs:6)
- **Template**: [`admin/dashboard.html.j2`](static/templates/admin/dashboard.html.j2)
- **Authentification**: Admin requis
- **Description**: Tableau de bord administrateur

#### `/mercure/admin/register` (GET)
- **Méthode**: [`register_get()`](src/routes/front_end/admin.rs:35)
- **Template**: [`admin/register.html.j2`](static/templates/admin/register.html.j2)
- **Authentification**: Admin requis
- **Description**: Page d'inscription d'un nouvel utilisateur

#### `/mercure/admin/passedit/<user_id>` (GET)
- **Méthode**: [`password_edit_get()`](src/routes/front_end/admin.rs:18)
- **Template**: [`admin/passedit.html.j2`](static/templates/admin/passedit.html.j2)
- **Authentification**: Admin requis ou utilisateur propriétaire
- **Description**: Page de modification de mot de passe

#### `/mercure/admin/editusers` (GET)
- **Méthode**: [`edit_users()`](src/routes/front_end/editusers.rs:6)
- **Template**: [`admin/users_list.html.j2`](static/templates/admin/users_list.html.j2)
- **Authentification**: Admin requis
- **Description**: Page de gestion des utilisateurs

#### `/mercure/admin/groupedit/<user_id>` (GET)
- **Méthode**: [`edit_groups_for_user()`](src/routes/front_end/editgroups.rs:7)
- **Template**: [`admin/groups_list.html.j2`](static/templates/admin/groups_list.html.j2)
- **Authentification**: Admin requis
- **Description**: Page d'édition des groupes pour un utilisateur

### Gestion des formulaires (Frontend)

#### `/mercure/admin/newform` (GET)
- **Méthode**: [`newform_get()`](src/routes/front_end/newform.rs:44)
- **Template**: [`admin/newform.html.j2`](static/templates/admin/newform.html.j2)
- **Authentification**: Admin requis
- **Description**: Page de création d'un nouveau formulaire

#### `/mercure/admin/editform/<formid>` (GET)
- **Méthode**: [`editform_get()`](src/routes/front_end/formedit.rs:13)
- **Template**: [`admin/newform.html.j2`](static/templates/admin/newform.html.j2)
- **Authentification**: Admin requis
- **Description**: Page d'édition d'un formulaire existant

#### `/mercure/admin/show/forms` (GET)
- **Méthode**: [`show_forms_get()`](src/routes/front_end/formedit.rs:34)
- **Template**: [`admin/showforms.html.j2`](static/templates/admin/showforms.html.j2)
- **Authentification**: Admin requis
- **Description**: Page listant tous les formulaires

### Gestion des runs (Frontend)

#### `/mercure/runs/submit/<form_id>` (GET)
- **Méthode**: [`new_run_get()`](src/routes/front_end/submitrun.rs:11)
- **Template**: [`common/newrun.html.j2`](static/templates/common/newrun.html.j2)
- **Authentification**: Utilisateur connecté
- **Description**: Page de soumission d'un nouveau run

#### `/mercure/show/runs/<run_id>` (GET)
- **Méthode**: [`show_run_get()`](src/routes/front_end/showrun.rs:9)
- **Template**: [`common/run.html.j2`](static/templates/common/run.html.j2)
- **Authentification**: Utilisateur connecté
- **Description**: Page d'affichage d'un run spécifique

#### `/mercure/show/runs` (GET)
- **Méthode**: [`show_runs_get()`](src/routes/front_end/showrun.rs:14)
- **Template**: [`common/showruns.html.j2`](static/templates/common/showruns.html.j2)
- **Paramètres**: `page` (optionnel)
- **Authentification**: Utilisateur connecté
- **Description**: Page listant tous les runs avec pagination

### Pages d'erreur

#### Templates d'erreur disponibles:
- [`errors/unauthorized.html.j2`](static/templates/errors/unauthorized.html.j2) - Accès non autorisé
- [`errors/admin_only.html.j2`](static/templates/errors/admin_only.html.j2) - Accès admin requis
- [`common/error.html.j2`](static/templates/common/error.html.j2) - Erreur générique

## Structures de données principales

### ApiResponse
Structure générique pour toutes les réponses API:
```json
{
  "success": true,
  "data": "T" | null,
  "error": "string" | null
}
```

### User
```json
{
  "id": 1,
  "usermail": "string",
  "username": "string",
  "created_at": "datetime",
  "last_login": "datetime|null",
  "is_admin": false
}
```

### Group
```json
{
  "id": 1,
  "name": "string"
}
```

### UserDefinedVar
```json
{
  "FromValuesList": {
    "allowed": ["value1", "value2"]
  }
}
// ou
{
  "Constant": "fixed_value"
}
// ou
"RunDefined"
```

### HgFormDef
```json
{
  "pipeline_name": "string",
  "launcher_name": "string",
  "form_name": "string",
  "enabled": true,
  "version": 1,
  "groups": [Group],
  "user_defined_vars": {
    "variable_name": UserDefinedVar
  }
}
```

### HgRunSubmission
```json
{
  "form_id": 1,
  "user_id": 1,
  "run_name": "string",
  "run_date": "string",
  "run_sequencer": "string",
  "run_flowcellid": "string",
  "sample_sheet_adn_path": "string",
  "sample_sheet_arn_path": "string",
  "metadata_path": "string",
  "user_defined_vars": {
    "key": "value"
  }
}
```

## Conventions de routage

- **Backend API**: Préfixe `/mercure/api/` + méthodes POST/GET retournant du JSON
- **Frontend**: Préfixe `/mercure/` + méthodes GET retournant du HTML
- **Admin**: Routes avec préfixe `/mercure/admin/` nécessitent une authentification admin
- **Authentification**: Gérée via cookies sécurisés avec garde [`Authenticated`](src/auth/guard.rs)
- **Templates**: Utilisation de Jinja2 avec répertoire [`static/templates/`](static/templates/)