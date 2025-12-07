# Procédure de lancement

## Etape 1: nginx doit être actif

Sur le serveur de production, `hmn-genseq-p01`, nginx est un service actif, défini dans une unité de systemd.

Pour savoir si nginx est actif:

```bash
singularity instance list | grep nginx
```
Si le résultat n'est pas vide, activer l'instance nginx.

En tant que root:
```bash
mkdir -p /var/log/nginx /var/cache/nginx /var/run
singularity instance start -B /var/log -B /var/cache -B /var/run nginx.sif nginx
```

## Etape 2: lancer mercure



# Configuration

J'ai créé deux configurations, une de développement (qui fait tourner tous les outils en local), et une configuration de production, qui permet d'écouter sur l'adresse IP du serveur de production.

## Configuration de développement

Cette configuration repose sur `nginx-test.conf`, présent dans `nginx-test.sif`, défini par `nginx-test.def`.
Pour ce qui est de l'application web mercure, c'est la compilation en mode debug qui permet d'accéder à ces options (voir la section debug de Rocket.toml).

Dans cette configuration:
- mercure écoute à l'adresse 127.0.0.1
- nginx redirige le flux de 

