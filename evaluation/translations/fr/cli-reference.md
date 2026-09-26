# <a id="ctsync-command-reference"></a>Référence de la commande `ctsync`

`ctsync` est l’équivalent en ligne de commande de Contoso Sync.

## Synopsis

```
ctsync <command> [--profile <name>] [--verbose]
```

## <a id="commands"></a>Commandes

| Commande | Description |
|---|---|
| `ctsync status` | Affiche l’état de synchronisation de chaque dossier |
| `ctsync pause --minutes 30` | Suspend la synchronisation pendant la durée indiquée |
| `ctsync resume` | Reprend une synchronisation suspendue |
| `ctsync reset --force` | Supprime le cache local et télécharge à nouveau tout le contenu |
| `ctsync logs --tail` | Affiche en continu le fichier journal `ctsync.log` |

## <a id="global-options"></a>Options globales

- `--profile <name>` : utilise le profil nommé défini dans `~/.ctsync/config.toml`.
- `--verbose` : affiche la sortie de débogage. Combinez cette option avec `--no-color` lorsque vous redirigez la sortie vers un fichier.
- `--json` : affiche une sortie lisible par ordinateur.

## <a id="exit-codes"></a>Codes de sortie

| Code | Signification |
|---|---|
| 0 | Réussite |
| 1 | Erreur générale |
| 3 | Non connecté |
| 7 | Le dossier est verrouillé par un autre processus |

## <a id="examples"></a>Exemples

Suspendez la synchronisation pendant une présentation :

```bash
ctsync pause --minutes 60
```

Vérifiez l’état d’un seul dossier et enregistrez-le au format JSON :

```bash
ctsync status --json > status.json
```
