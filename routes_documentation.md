# Documentation des Routes de l'Application Web Mercure

Cette application web Rust utilise le framework Rocket avec des routes séparées entre backend (API) et frontend (rendu de templates).

## Routes Backend (API)

Les routes backend utilisent principalement des méthodes POST pour les opérations de données et retournent du JSON.

### Authentification

#### `/mercure/api/login` (POST)

- **Méthode**: [`login_post()`](mercure-lib/src/routes/back_end/login.rs:18)
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

- **Méthode**: [`logout_get()`](mercure-lib/src/routes/back_end/logout.rs:6)
- **Description**: Déconnecte l'utilisateur et supprime la session
- **Retour**: Redirection vers `/mercure`

### Gestion des utilisateurs

#### `/mercure/api/register` (POST)

- **Méthode**: [`register_post()`](mercure-lib/src/routes/back_end/newuser.rs:12)
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

- **Méthode**: [`list_users()`](mercure-lib/src/routes/back_end/listusers.rs:6)
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

- **Méthode**: [`password_edit_post()`](mercure-lib/src/routes/back_end/passedit.rs:8)
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

- **Méthode**: [`list_groups()`](mercure-lib/src/routes/back_end/listgroups.rs:6)
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

- **Méthode**: [`list_groups_for_user()`](mercure-lib/src/routes/back_end/listgroups.rs:14)
- **Description**: Liste les groupes d'un utilisateur spécifique

#### `/mercure/api/newgroup/<group_name>` (GET)

- **Méthode**: [`newgroup_get()`](mercure-lib/src/routes/back_end/newgroup.rs:10)
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

- **Méthode**: [`update_groups()`](mercure-lib/src/routes/back_end/editgroups.rs:14)
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

- **Méthode**: [`get_all_forms()`](mercure-lib/src/routes/back_end/getform.rs:10)
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

- **Méthode**: [`get_all_forms_for_group()`](mercure-lib/src/routes/back_end/getform.rs:18)
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

- **Méthode**: [`get_form_from_id()`](mercure-lib/src/routes/back_end/getform.rs:32)
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
    "indir_type": "BclDir|AnalysisDir|OntDir",
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

- **Méthode**: [`newform_post()`](mercure-lib/src/routes/back_end/newform.rs:52)
- **Authentification**: Admin requis
- **Schéma JSON**: Même structure que le retour de `/mercure/api/forms/<id>`
- **Description**: Création d'un nouveau formulaire

#### `/mercure/api/enable/<formid>` (GET)

- **Méthode**: [`enable_form()`](mercure-lib/src/routes/back_end/newform.rs:32)
- **Authentification**: Admin requis
- **Description**: Active un formulaire

#### `/mercure/api/disable/<formid>` (GET)

- **Méthode**: [`disable_form()`](mercure-lib/src/routes/back_end/newform.rs:11)
- **Authentification**: Admin requis
- **Description**: Désactive un formulaire

### Gestion des runs

#### `/mercure/api/newrun` (POST)

- **Méthode**: [`newrun_post()`](mercure-lib/src/routes/back_end/submitrun.rs:15)
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
  },
  "indir": "string|null",
  "outdir": "string|null"
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

#### `/mercure/api/editrun` (POST)

- **Méthode**: [`editrun_post()`](mercure-lib/src/routes/back_end/submitrun.rs:45)
- **Schéma JSON**:

```json
{
  "run_id": 1,
  "run_date": "string",
  "run_sequencer": "string",
  "run_flowcellid": "string",
  "sample_sheet_adn_path": "string",
  "sample_sheet_arn_path": "string",
  "metadata_path": "string",
  "user_defined_vars": {
    "key": "value"
  },
  "indir": "string|null",
  "outdir": "string|null"
}
```

- **Description**: Modification d'un run existant (uniquement si statut = Idle)
- **Retour JSON**:

```json
{
  "success": true,
  "data": {
    "message": "Run modifié avec succès!",
    "run_id": 1
  }
}
```

#### `/mercure/api/validate/<run_id>` (POST)

- **Méthode**: [`validate_run_post()`](mercure-lib/src/routes/back_end/submitrun.rs:30)
- **Description**: Valide un run (transition Idle → Pending) et crée une nouvelle tentative
- **Retour JSON**:

```json
{
  "success": true,
  "data": {
    "message": "Run validé avec succès!",
    "run_id": 1
  }
}
```

#### `/mercure/api/listruns?<page>&<page_size>&<status>` (GET)

- **Méthode**: [`list_runs_get()`](mercure-lib/src/routes/back_end/listruns.rs:194)
- **Paramètres**:
  - `page`: Numéro de page (optionnel, défaut: 1)
  - `page_size`: Nombre d'éléments par page (optionnel, défaut: 5)
  - `status`: Filtre par statut (optionnel)
- **Description**: Liste les runs avec pagination
- **Retour JSON**:

```json
{
  "success": true,
  "data": {
    "title": "string",
    "header": [...],
    "table": [...],
    "pagination": {
      "current_page": 1,
      "total_pages": 10,
      "page_size": 5,
      "total_count": 50
    }
  }
}
```

#### `/mercure/api/searchrun?<page>&<page_size>&<status>&<date_from>&<date_to>&<run_name_search>` (GET)

- **Méthode**: [`search_run_get()`](mercure-lib/src/routes/back_end/listruns.rs:246)
- **Paramètres**:
  - `page`: Numéro de page (optionnel)
  - `page_size`: Nombre d'éléments par page (optionnel)
  - `status`: Filtre par statut (optionnel)
  - `date_from`: Date de début (optionnel)
  - `date_to`: Date de fin (optionnel)
  - `run_name_search`: Recherche par nom de run (optionnel)
- **Description**: Recherche de runs avec filtres avancés et pagination
- **Retour JSON**: Même structure que `/mercure/api/listruns`

### Monitoring et surveillance

#### `/mercure/api/watch/<job_id>/<attempt_number>` (GET)

- **Méthode**: [`watch()`](mercure-lib/src/routes/back_end/jobprogresswatch.rs:10)
- **Description**: Surveillance en temps réel de l'état d'exécution d'un job
- **Retour JSON**:

```json
{
  "success": true,
  "data": {
    "logs": "...",
    "status": "...",
    "progress": "..."
  }
}
```

### Utilitaires

#### `/mercure/api/prettify/seqname/<seqname>` (GET)

- **Méthode**: [`prettify_seqname()`](mercure-lib/src/routes/back_end/prettify.rs:4)
- **Description**: Récupère le nom formaté d'un séquenceur
- **Retour**: String (nom formaté)

#### `/mercure/api/redirect?<message>&<title>&<target_url>&<seconds>` (GET)

- **Méthode**: [`redirect_post()`](mercure-lib/src/routes/back_end/redirect.rs:15)
- **Paramètres**:
  - `message`: Message à afficher
  - `title`: Titre de la page
  - `target_url`: URL de redirection
  - `seconds`: Délai de redirection (optionnel, défaut: 5)
- **Description**: Page de redirection avec délai
- **Retour**: Template HTML de redirection

### Upload de fichiers

#### `/mercure/api/upload` (POST)

- **Méthode**: [`upload_post()`](mercure-lib/src/routes/back_end/upload.rs:15)
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

### Gestion des SampleSheets

#### `/mercure/api/samplesheet/listsamples` (POST)

- **Méthode**: [`list_samples_from_samplesheet()`](mercure-lib/src/routes/back_end/samplesheet.rs:18)
- **Format**: Form data
- **Champs**:
  - `file_name`: Chemin vers le fichier SampleSheet
- **Description**: Extrait la liste des échantillons d'une SampleSheet
- **Retour JSON**:

```json
{
  "success": true,
  "data": {
    "sample_names": ["sample1", "sample2", ...]
  }
}
```

#### `/mercure/api/samplesheet/check` (POST)

- **Méthode**: [`check_samplesheet()`](mercure-lib/src/routes/back_end/samplesheet.rs:36)
- **Format**: Form data
- **Champs**:
  - `file_name`: Chemin vers le fichier SampleSheet
- **Description**: Valide et corrige si nécessaire une SampleSheet Illumina
- **Retour JSON**:

```json
{
  "success": true,
  "data": {
    "message": "string (HTML formatted)",
    "check": "perfect|fixable|invalid",
    "sample_names": [...],
    "unique_sample_count": 10,
    "file_name": "string"
  }
}
```

### Gestion des répertoires

#### `/mercure/api/directories/list?<dirtype>` (GET)

- **Méthode**: [`list_directories_by_type()`](mercure-lib/src/routes/back_end/directory_listing.rs:12)
- **Paramètres**:
  - `dirtype`: Type de répertoire ("analysis" ou "ont")
- **Authentification**: Utilisateur connecté requis
- **Description**: Liste les répertoires disponibles par type
- **Retour JSON**:

```json
{
  "success": true,
  "data": [
    {
      "name": "directory_name",
      "path": "/full/path/to/directory"
    }
  ]
}
```

## Routes Frontend (Rendu de templates)

Les routes frontend utilisent principalement des méthodes GET et retournent du HTML via des templates Jinja2.

### Pages génériques

#### `/mercure` (GET)

- **Méthode**: [`welcome_page_get()`](mercure-lib/src/routes/front_end/generics.rs:6)
- **Template**: [`common/welcome.html.j2`](static/templates/common/welcome.html.j2)
- **Description**: Page d'accueil de l'application

#### `/mercure/login` (GET)

- **Méthode**: [`login_get()`](mercure-lib/src/routes/front_end/generics.rs:11)
- **Template**: [`common/login.html.j2`](static/templates/common/login.html.j2)
- **Description**: Page de connexion

#### `/mercure/success` (GET)

- **Méthode**: [`success_page_get()`](mercure-lib/src/routes/front_end/generics.rs:16)
- **Template**: [`common/success.html.j2`](static/templates/common/success.html.j2)
- **Paramètres**: `origin`, `message`
- **Description**: Page de confirmation d'action

#### `/mercure/home` (GET)

- **Méthode**: [`home_get()`](mercure-lib/src/routes/front_end/home.rs:6)
- **Template**: [`common/home.html.j2`](static/templates/common/home.html.j2)
- **Authentification**: Utilisateur connecté
- **Description**: Page d'accueil utilisateur avec liste des groupes

### Administration

#### `/mercure/admin/dashboard` (GET)

- **Méthode**: [`admin_dashboard_get()`](mercure-lib/src/routes/front_end/admin.rs:6)
- **Template**: [`admin/dashboard.html.j2`](static/templates/admin/dashboard.html.j2)
- **Authentification**: Admin requis
- **Description**: Tableau de bord administrateur

#### `/mercure/admin/register` (GET)

- **Méthode**: [`register_get()`](mercure-lib/src/routes/front_end/admin.rs:35)
- **Template**: [`admin/register.html.j2`](static/templates/admin/register.html.j2)
- **Authentification**: Admin requis
- **Description**: Page d'inscription d'un nouvel utilisateur

#### `/mercure/admin/passedit/<user_id>` (GET)

- **Méthode**: [`password_edit_get()`](mercure-lib/src/routes/front_end/admin.rs:18)
- **Template**: [`admin/passedit.html.j2`](static/templates/admin/passedit.html.j2)
- **Authentification**: Admin requis ou utilisateur propriétaire
- **Description**: Page de modification de mot de passe

#### `/mercure/admin/editusers` (GET)

- **Méthode**: [`edit_users()`](mercure-lib/src/routes/front_end/editusers.rs:6)
- **Template**: [`admin/users_list.html.j2`](static/templates/admin/users_list.html.j2)
- **Authentification**: Admin requis
- **Description**: Page de gestion des utilisateurs

#### `/mercure/admin/groupedit/<user_id>` (GET)

- **Méthode**: [`edit_groups_for_user()`](mercure-lib/src/routes/front_end/editgroups.rs:7)
- **Template**: [`admin/groups_list.html.j2`](static/templates/admin/groups_list.html.j2)
- **Authentification**: Admin requis
- **Description**: Page d'édition des groupes pour un utilisateur

### Gestion des formulaires (Frontend)

#### `/mercure/admin/newform` (GET)

- **Méthode**: [`newform_get()`](mercure-lib/src/routes/front_end/newform.rs:44)
- **Template**: [`admin/editform.html.j2`](static/templates/admin/editform.html.j2)
- **Authentification**: Admin requis
- **Description**: Page de création d'un nouveau formulaire

#### `/mercure/admin/editform/<formid>` (GET)

- **Méthode**: [`editform_get()`](mercure-lib/src/routes/front_end/formedit.rs:13)
- **Template**: [`admin/editform.html.j2`](static/templates/admin/editform.html.j2)
- **Authentification**: Admin requis
- **Description**: Page d'édition d'un formulaire existant

#### `/mercure/admin/show/forms` (GET)

- **Méthode**: [`show_forms_get()`](mercure-lib/src/routes/front_end/formedit.rs:34)
- **Template**: [`admin/showforms.html.j2`](static/templates/admin/showforms.html.j2)
- **Authentification**: Admin requis
- **Description**: Page listant tous les formulaires

### Gestion des runs (Frontend)

#### `/mercure/runs/submit/<form_id>` (GET)

- **Méthode**: [`new_run_get()`](mercure-lib/src/routes/front_end/submitrun.rs:11)
- **Template**: [`common/newrun.html.j2`](static/templates/common/newrun.html.j2)
- **Authentification**: Utilisateur connecté
- **Description**: Page de soumission d'un nouveau run

#### `/mercure/editrun/<run_id>` (GET)

- **Méthode**: [`edit_run_get()`](mercure-lib/src/routes/front_end/editrun.rs:17)
- **Template**: [`common/newrun.html.j2`](static/templates/common/newrun.html.j2) (mode édition)
- **Authentification**: Utilisateur connecté avec permissions sur le run
- **Description**: Page d'édition d'un run existant (uniquement si statut = Idle)

#### `/mercure/show/run/<run_id>?<attempt_number>` (GET)

- **Méthode**: [`show_run_get()`](mercure-lib/src/routes/front_end/showrun.rs:19)
- **Templates**: Variables selon le statut:
  - [`common/idlerun.html.j2`](static/templates/common/idlerun.html.j2) - Run Idle (mode édition)
  - [`common/pendingrun.html.j2`](static/templates/common/pendingrun.html.j2) - Run en attente
  - [`common/runningrun.html.j2`](static/templates/common/runningrun.html.j2) - Run en cours
  - [`common/successrun.html.j2`](static/templates/common/successrun.html.j2) - Run réussi
  - [`common/failurerun.html.j2`](static/templates/common/failurerun.html.j2) - Run échoué
- **Paramètres**:
  - `attempt_number`: Numéro de tentative à afficher (optionnel, défaut: dernière tentative)
- **Authentification**: Utilisateur connecté avec permissions sur le run
- **Description**: Page d'affichage détaillé d'un run avec historique des tentatives

#### `/mercure/show/runs` (GET)

- **Méthode**: [`list_runs()`](mercure-lib/src/routes/front_end/showrun.rs:258)
- **Template**: [`common/listruns.html.j2`](static/templates/common/listruns.html.j2)
- **Authentification**: Utilisateur connecté
- **Description**: Page de listing des runs avec pagination (utilise l'API `/mercure/api/listruns`)

#### `/mercure/search/run` (GET)

- **Méthode**: [`search_run()`](mercure-lib/src/routes/front_end/showrun.rs:263)
- **Template**: [`common/searchrun.html.j2`](static/templates/common/searchrun.html.j2)
- **Authentification**: Utilisateur connecté
- **Description**: Page de recherche de runs avec filtres avancés

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

### DirectoryInfo

```json
{
  "name": "string",
  "path": "string"
}
```

### RunStatus

Énumération des statuts de run:

- `Idle`: À valider
- `Pending`: En attente d'exécution
- `Running`: En cours d'exécution
- `Success`: Terminé avec succès
- `Failure(String)`: Échoué avec raison

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

### IndirType

Énumération des types de répertoires d'entrée:

- `BclDir`: Répertoire BCL (défaut)
- `AnalysisDir`: Répertoire d'analyse
- `OntDir`: Répertoire ONT

### HgFormDef

```json
{
  "form_id": 1,
  "pipeline_name": "string",
  "launcher_name": "string",
  "form_name": "string",
  "enabled": true,
  "version": 1,
  "groups": [Group],
  "indir_type": "BclDir|AnalysisDir|OntDir",
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
  },
  "indir": "string|null",
  "outdir": "string|null"
}
```

### HgRunEdit

```json
{
  "run_id": 1,
  "run_date": "string",
  "run_sequencer": "string",
  "run_flowcellid": "string",
  "sample_sheet_adn_path": "string",
  "sample_sheet_arn_path": "string",
  "metadata_path": "string",
  "user_defined_vars": {
    "key": "value"
  },
  "indir": "string|null",
  "outdir": "string|null"
}
```

### HgRun

```json
{
  "run_id": 1,
  "form": HgFormDef,
  "user": User,
  "user_defined_vars": {
    "key": "value"
  },
  "run_name": "string",
  "run_date": "string",
  "creation_date": "string",
  "run_sequencer": "string",
  "run_flowcellid": "string",
  "sample_sheet_adn_path": "string",
  "sample_sheet_arn_path": "string",
  "metadata_path": "string",
  "indir": "string|null",
  "outdir": "string|null",
  "status": RunStatus,
  "attempt_count": 1
}
```

### HgAttempt

```json
{
  "attempt_number": 1,
  "run_id": 1,
  "attempt_date": "datetime",
  "user_defined_vars": {
    "key": "value"
  },
  "run_date": "string",
  "run_sequencer": "string",
  "run_flowcellid": "string",
  "sample_sheet_adn_path": "string",
  "sample_sheet_arn_path": "string",
  "metadata_path": "string",
  "indir": "string|null",
  "outdir": "string|null",
  "status": RunStatus,
  "comment": "string"
}
```

## Conventions de routage

- **Backend API**: Préfixe `/mercure/api/` + méthodes POST/GET retournant du JSON
- **Frontend**: Préfixe `/mercure/` + méthodes GET retournant du HTML
- **Admin**: Routes avec préfixe `/mercure/admin/` nécessitent une authentification admin
- **Authentification**: Gérée via cookies sécurisés avec garde [`Authenticated`](mercure-lib/src/auth/guard.rs)
- **Templates**: Utilisation de Jinja2 avec répertoire [`static/templates/`](static/templates/)

## Machine d'état des Runs

L'application implémente une machine d'état stricte pour les runs avec les transitions suivantes:

1. **Idle → Pending**: Validation du formulaire (`/mercure/api/validate/<run_id>`)
2. **Pending → Running**: Démarrage de l'analyse (automatique via routine)
3. **Running → Success**: Fin avec succès (automatique via routine)
4. **Running → Failure**: Fin avec échec (automatique via routine)
5. **Success/Failure → Idle**: Relance du run (admin uniquement)

Chaque transition de **Idle → Pending** crée une nouvelle tentative ([`HgAttempt`](mercure-lib/src/models/attempt.rs:15)) qui conserve un instantané des données du run au moment de la validation.
