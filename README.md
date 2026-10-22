# Qu'est-ce que ce logiciel

Le logiciel `smaug` est une application web couplée à une routine d'exécution automatique de scripts.

Cette application permet d'exécuter des tâches définies dans un répertoire git web (pour l'instant seul gitea est supporté).


# Instructions d'installation

## Prérequis

### Logiciels

- Linux (support de windows envisageable, mais pas prévu pour le moment)
- nginx (ou autre reverse-proxy, optionnel)

### Environnement web

- un dépôt gitea accessible depuis le serveur qui exécute smaug

### Système de fichiers

Fichiers et dossiers de travail:

- Un dossier pour stocker les logs d'exécution des scripts lancés par la routine
- Un dossier pour stocker les scripts exécutés
- Un dossier pour stocker les fichiers uploadés par l'utilisateur
- Un fichier `Rocket.toml` pour configurer l'application

Pour des raisons pratiques, il peut être judicieux de créer un seul dossier, par exemple dans `/opt/webapps/smaug`, et d'y placer tous ces dossiers ainsi que le fichier `Rocket.toml`.

Fichiers d'installation:

- Un fichier `/etc/systemd/system/smaug.service` (optionnel mais très pratique): permet de servir smaug en permanence, à chaque démarrage.
- L'exécutable lui-même, qui peut être stocké n'importe où (attention aux permissions, notamment SELinux, pour l'utiliser avec systemd). Emplacement recommandé: `/usr/local/bin/smaug`.


## Etape 1 (optionnelle): Installation et configuration de nginx


Installer et configurer nginx sur le serveur de production. Au choix, soit en dur, soit dans une image podman, ou singularity.

Quel que soit le choix, voici une configuration nginx qui fonctionne, pour servir smaug sur son IP/port par défaut:

```conf
location /smaug {
    proxy_pass http://127.0.0.1:8000/smaug;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
}
```

Remplacer `:8000` par le port que vous utilisez effectivement (voir les options de smaug web).
Il est conseillé de garder `127.0.0.1` comme adresse d'écoute au lieu de l'IP que vous voulez exposer, cela permet d'éviter une connexion directe sans passer par le reverse-proxy.

## Etape 2: Installation de smaug


### Installation du binaire

Télécharger le binaire adapté à votre architecture (pour l'instant, seulement x86_64 avec GLIBC 2.39, ou x86_64 avec musl).

Le plus compatible est la version musl, quoiqu'un peu moins performante que la version glibc.
Avec musl, l'avantage est que le binaire est entièrement statique (aucune dépendance aux bibliothèques de l'OS, pas même GLIBC).

Sinon, le compiler avec la version de rust du système:

```bash
cargo build --release
```

En tant que root, écrire le fichier `smaug` dans `/usr/local/bin`

```bash
mv smaug /usr/local/bin
# S'assurer qu'il est exécutable
chmod +x /usr/local/bin/smaug
```

Si vous utilisez SELinux et que le contexte de `/usr/local/bin/smaug` est incorrect:

```bash
restorecon /usr/local/bin/smaug
```

Le binaire est prêt à être exécuté.

### Configuration

L'application est configurée à l'aide d'un fichier unique, `Rocket.toml`. En mode release, l'option `secret_key` est obligatoire.

Ci-dessous, un exemple de configuration valide, à adapter:

```toml
[release]
# Web
secret_key = "$(openssl rand -base64 32)"
port = 8000
address = "127.0.0.1"

# Token de mise à jour des pipelines
webhook_secret = "$(openssl rand -base64 32)"

# Fonctionnement spécifique de smaug
smaug_db = "sqlite:///OPT/webapps/smaug/smaug.db?mode=rwc"
upload_dir = "/OPT/webapps/smaug/uploads"
logs_dir = "/OPT/webapps/smaug/logs"
analysis_dir = "/data/analysis"

[release.pipelines]
# Configuration de la base de vérité des pipelines
api = "gitea_v1" # Pour l'instant, seule l'API gitea v1 est supportée
base_url = "http://localhost/gitea" # Pas uniquement localhost, n'importe quelle URL de base valide avec l'API v1 de gitea disponible
owner = "bioinfo"
repo = "pipelines" # Un dépôt de confiance
```

### Définition du service

Créer un fichier `/etc/systemd/system/smaug.service` avec le contenu suivant:

```ini
[Unit]
Description=Smaug web app
After=network.target

[Service]
Type=simple
User=hermes
ExecStart=/usr/local/bin/smaug web -p 8000 -H 127.0.0.1
WorkingDirectory=/OPT/webapps/smaug
Restart=always

[Install]
WantedBy=multi-user.target
```

Recharger les services:

```bash
systemctl daemon-reload
```

Activer smaug

```bash
systemctl enable smaug
```

## Etape 3: lancer smaug

Lancer smaug

```bash
systemctl start smaug
```


