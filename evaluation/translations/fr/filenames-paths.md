# <a id="files-and-folders"></a>Fichiers et dossiers

Le client stocke ses paramètres dans `settings.json` et ses journaux dans le dossier `logs`. Sous Windows, le chemin complet est `C:\Users\<name>\AppData\Local\ContosoSync\logs\ctsync.log` ; sous Linux, il s’agit de `~/.local/share/ctsync/ctsync.log`.

Pour réinitialiser le client, supprimez settings.json et redémarrez-le. Ne supprimez pas sync.db, car il contient la liste des fichiers synchronisés.

Ouvrez le fichier README.md à la racine du dépôt et suivez les étapes. Le script ./scripts/install.sh installe les dépendances, et deploy.ps1 publie la build dans le dossier dist/.

Les images telles que logo.png, banner@2x.jpg et icons/app.ico sont copiées telles quelles.

| Fichier | Rôle |
|---|---|
| `config.toml` | Configuration principale |
| `ignore.txt` | Motifs à exclure |
| `Dockerfile` | Build de conteneur |
| `.env.example` | Modèle pour les variables d’environnement |

Les fichiers journaux datant de plus de sept jours sont compressés en `ctsync-2026-08-01.log.gz`.
