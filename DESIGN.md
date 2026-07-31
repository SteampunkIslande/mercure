# Mercure — Design Document

## Overview

Mercure is a web application whose primary purpose is to **generate a bash script** that will be executed on a compute cluster, and to **report on its progress** by parsing its logs from a predefined location.

Advanced users (pipeline authors) define **YAML files** that describe the required variables to generate the final script. They also write **script templates** using **minijinja** syntax that reference both user-provided variables and pre-defined Mercure variables (e.g. `mercure.indir`, `mercure.outdir`, `mercure.run_name`).

---

## Pipelines Repository

### Structure

All pipelines are stored in a single **git-managed folder** (the "pipelines repo"). Each subfolder is a pipeline:

```
pipelines/
├── pipeline_a/
│   ├── forms.yaml          # special file defining the forms (see below)
│   ├── templates/
│   │   ├── script_a.sh.j2  # minijinja template
│   │   └── script_b.sh.j2
│   └── ...
├── pipeline_b/
│   ├── forms.yaml
│   └── templates/
│       └── script_c.sh.j2
└── ...
```

### Git versioning

- The entire pipelines folder is a git repository.
- The repository is served over **HTTP at a configurable URL**, so any file can be retrieved at any commit.
- The routine process periodically runs `git pull` on the pipelines repo (only when no runs are pending), to keep it up to date.

### Dev mode vs Production mode

When an admin user creates a form, they can choose to make it **"dev mode"**:

- **Dev mode**: The admin is prompted for a **branch name**. Every time this branch is updated, the form will always check out the **latest commit** of that branch when generating scripts.
- **Production mode**: The form definition pins a **specific commit hash**. Scripts generated from this form always check out that exact commit, ensuring reproducibility.

### Clone into outdir

No matter what the template script content is, Mercure will **always `git clone` the entire pipelines repo** into `mercure.outdir/pipelines/` and:

- **Dev mode**: checkout the designated branch.
- **Production mode**: checkout the specific commit hash defined in the form definition.

This ensures the full pipeline code is available at run time, inside the output directory.

---

## Form Definition (YAML)

A special YAML file at the root of each pipeline directory (e.g. `forms.yaml`) defines the forms. It contains a **list of objects**, each one defining:

1. **`template`**: The file name to use as a template for generating the script. Must be **relative** and within the pipeline directory.
2. **`variables`**: A list of required user variables. Each variable specifies its **type**:
   - **`file`**: An existing file the user should upload.
   - **`choice`**: A value selected among several predefined options.
   - **`constant`**: A fixed value (not editable by the user). Allows the same template to be reused in multiple forms with different constant values.

A given template can be used in **several forms**, hence the **constant** variable type: the same template can be instantiated with different fixed values depending on the form.

### Example YAML

```yaml
forms:
  - name: "RNASeq Standard"
    template: "templates/rnaseq.sh.j2"
    dev_mode: false
    variables:
      - name: "species"
        type: "choice"
        choices: ["human", "mouse"]
      - name: "project"
        type: "constant"
        value: "CancerStudy"
      - name: "samplesheet"
        type: "file"
      - name: "batch"
        type: "value"  # free-text input from the user

  - name: "RNASeq Dev"
    template: "templates/rnaseq.sh.j2"
    dev_mode: true
    dev_branch: "feature/new-aligner"
    variables:
      - name: "species"
        type: "choice"
        choices: ["human", "mouse", "rat"]
      - name: "project"
        type: "constant"
        value: "PilotStudy"
      - name: "samplesheet"
        type: "file"
```

---

## Script Templates (minijinja)

Templates use **minijinja** syntax and have access to:

- **User-defined variables**: the values provided by the user through the form (file uploads, choices, free text, constants).
- **Pre-defined Mercure variables** (namespaced under `mercure`):
  - `mercure.indir` — input directory for the run
  - `mercure.outdir` — output directory for the run
  - `mercure.run_name` — user-defined name for the run
  - `mercure.run_id` — internal run ID
  - `mercure.attempt_number` — attempt number for this run
  - `mercure.pipelines_dir` — path to the cloned pipelines repo inside outdir

### Example template

```jinja2
#!/bin/bash

# Run: {{ mercure.run_name }}
# Input: {{ mercure.indir }}
# Output: {{ mercure.outdir }}

species="{{ species }}"
project="{{ project }}"
samplesheet="{{ samplesheet }}"

cd {{ mercure.pipelines_dir }}

bash pipeline.sh \
  --species {{ species }} \
  --project {{ project }} \
  --samplesheet {{ samplesheet }} \
  --indir {{ mercure.indir }} \
  --outdir {{ mercure.outdir }}
```

---

## Script Generation

When a user submits a run:

1. Mercure reads the form definition (YAML) for the selected form.
2. Collects user-provided variable values (file paths, choices, free text).
3. Injects constant values from the form definition.
4. Renders the minijinja template with all variables (user + Mercure pre-defined).
5. Writes the generated bash script to the jobs directory.
6. The routine process picks up the script and executes it.

---

## Log Parsing & Progress Reporting

- The generated script writes logs to a **predefined location** (`mercure.logs_dir`).
- Log files follow a naming convention: `job-{run_id:010}-{attempt_number:010}.log`.
- Mercure parses these logs to report on progress:
  - **Step tracking**: lines matching `## STEP <name>` indicate the current step.
  - **Progress**: lines matching `X of Y steps (Z%) done` provide percentage progress.
  - **SLURM job monitoring**: lines matching `SLURM run ID: <id>` allow querying `squeue` for job counts (pending, running, done).
  - **Error detection**: lines matching `## ERROR <message>` are collected as errors.
- A **Server-Sent Events (SSE) endpoint** streams real-time job info to the frontend.

---

## Run Lifecycle (State Machine)

```
Idle → Pending → Running → Success
                         ↘ Failure
```

- **Idle**: Run created, not yet validated.
- **Pending**: User has validated the form. The routine is looking for input data / waiting to start.
- **Running**: The script has been generated and submitted for execution.
- **Success**: The script completed with exit code 0.
- **Failure**: The script failed (exit code ≠ 0). Error messages are extracted from logs.

### Attempts

Each time a run transitions from Idle → Pending, a new **attempt** is created. Attempts are immutable snapshots of the run's editable fields at validation time. Multiple attempts can exist for a single run (re-runs).

---

## Authentication & Authorization

- Users authenticate with email/password (bcrypt hashing).
- **Admin** users can create/edit forms, manage groups, register users.
- **Groups** control which forms are visible to which users (form-group and group-user associations).
- Regular users can submit runs, view their runs, edit runs (before validation), and retry runs.

---

## Routine Process

A separate binary (`routine`) runs as a background service:

1. **Pending runs**: Looks for runs in `Pending` status. Checks if input data is available (e.g. sequencer output directory). If found and complete, generates the script and starts the analysis.
2. **Running runs**: Monitors running attempts by checking for script files in `TODO/`, `RUNNING/`, `DONE/`, `FAILS/` directories. Updates run status accordingly.
3. **Pipeline updates**: When no runs are pending, performs `git pull` on the pipelines repo to fetch updates.

---

## Configuration

Mercure is configured via `Rocket.toml` with a `MercureConfig` struct:

| Key                   | Description                                                                       |
| --------------------- | --------------------------------------------------------------------------------- |
| `mercure_db`          | SQLite database path                                                              |
| `pipeline_dir`        | Path to the git-managed pipelines repository                                      |
| `jobs_dir`            | Directory for generated scripts (subdirs: `TODO/`, `RUNNING/`, `DONE/`, `FAILS/`) |
| `logs_dir`            | Directory for log files                                                           |
| `upload_dir`          | Directory for user-uploaded files                                                 |
| `static_dir`          | Static web assets                                                                 |
| `sequencers_dir`      | Raw sequencer output directory                                                    |
| `analysis_dir`        | Analysis output directory                                                         |
| `ont_dir`             | ONT sequencer output directory                                                    |
| `check_run_completed` | Script to check if an Illumina run is complete                                    |
| `post_run_script`     | Script executed after the main script completes                                   |

---

## Resolved Design Decisions

1. **YAML + DB cache**: YAML files are the source of truth for form structure. The DB caches form definitions for fast lookups, tracks enabled/disabled status, and stores the pinned commit hash / dev branch.
2. **Fully replace launchers with minijinja**: Templates are pure minijinja `.j2` files. The launcher concept (bash with `##` comments concatenated into the generated script) is removed entirely.
3. **Four variable types**: `file` (upload), `choice` (dropdown), `constant` (fixed per-form), `value` (free-text input from the user, replaces `RunDefined`).
4. **External Gitea serves the pipelines repo over HTTP**: Mercure stores a configurable URL (`pipelines_repo_url`) pointing to the external Gitea instance. The repo is accessible at any commit via this URL.
5. **Incremental migration**: Refactor the existing Rust/Rocket code step by step, replacing the launcher approach with YAML + minijinja templates, keeping auth, DB, and routine mostly intact.

## Key Design Decisions Summary

1. **YAML-based form definitions** at the root of each pipeline directory, replacing the previous database-only approach.
2. **minijinja templates** for script generation, using both user variables and `mercure.*` pre-defined variables.
3. **Four variable types**: `file` (upload), `choice` (dropdown), `constant` (fixed per-form, enables template reuse), `value` (free-text).
4. **Dev mode** (branch tracking) vs **Production mode** (pinned commit hash) for pipeline version control.
5. **Always clone the pipelines repo** into `mercure.outdir/pipelines/` at the appropriate commit/branch.
6. **External Gitea** serves the pipelines repo over HTTP, enabling retrieval of any file at any commit.
7. **Log-based progress reporting** via SSE, parsing step markers, progress percentages, SLURM job states, and error lines.
8. **Git-managed pipelines folder** with periodic `git pull` by the routine process (only when idle).
9. **Run state machine** (Idle → Pending → Running → Success/Failure) with immutable attempt snapshots.
10. **Group-based access control** for form visibility.

# Choix des branches de travail

Pour créer un nouveau formulaire, l'utilisateur peut choisir une branche parmi celles qui existent, et pour ce faire, mercure peut lui lister les branches existantes:

```
http://hostname/api/v1/repos/{owner}/pipelines/branches
Retourne une liste d'objets, un par branche existante
```

```rust

// Code snippet to parse existing branches (listed using the above method)

use serde_derive::Deserialize;
use serde_derive::Serialize;
use serde_json::Value;

pub type BranchesList = Vec<BranchDef>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchDef {
    pub name: String,
    pub commit: Commit,
    pub protected: bool,
    pub required_approvals: i64,
    pub enable_status_check: bool,
    pub status_check_contexts: Vec<Value>,
    pub user_can_push: bool,
    pub user_can_merge: bool,
    pub effective_branch_protection_name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub message: String,
    pub url: String,
    pub author: Author,
    pub committer: Committer,
    pub verification: Verification,
    pub timestamp: String,
    pub added: Value,
    pub removed: Value,
    pub modified: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    pub email: String,
    pub username: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Committer {
    pub name: String,
    pub email: String,
    pub username: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Verification {
    pub verified: bool,
    pub reason: String,
    pub signature: String,
    pub signer: Value,
    pub payload: String,
}

```