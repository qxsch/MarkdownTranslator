# <a id="ctsync-command-reference"></a>Référence de la commande `ctsync`

`ctsync` est l’outil en ligne de commande associé à Contoso Sync.

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
| `ctsync logs --tail` | Diffuse en continu le fichier journal `ctsync.log` |

## <a id="global-options"></a>Options globales

- `--profile <name>` : utilise le profil portant le nom indiqué, défini dans `~/.ctsync/config.toml`.
- `--verbose` : affiche la sortie de débogage. Associez cette option à `--no-color` lorsque vous redirigez la sortie vers un fichier.
- `--json` : affiche une sortie lisible par machine.

## <a id="exit-codes"></a>Codes de sortie

| Code | Signification |
|---|---|
| 0 | Réussite |
| 1 | Erreur générale |
| 3 | Non connecté |
| 7 | Le dossier est verrouillé par un autre processus |

## <a id="examples"></a>Exemples

Suspendre la synchronisation pendant une présentation :

```bash
ctsync pause --minutes 60
```

Vérifier l’état d’un seul dossier et l’enregistrer au format JSON :

```bash
ctsync status --json > status.json
```
