# <a id="ctsync-command-reference"></a>`ctsync`-Befehlsreferenz

`ctsync` ist das Befehlszeilen-Begleittool von Contoso Sync.

## <a id="synopsis"></a>Syntax

```
ctsync <command> [--profile <name>] [--verbose]
```

## <a id="commands"></a>Befehle

| Befehl | Beschreibung |
|---|---|
| `ctsync status` | Zeigt den Synchronisierungsstatus jedes Ordners an |
| `ctsync pause --minutes 30` | Hält die Synchronisierung für die angegebene Zeit an |
| `ctsync resume` | Setzt eine angehaltene Synchronisierung fort |
| `ctsync reset --force` | Löscht den lokalen Cache und lädt alles erneut herunter |
| `ctsync logs --tail` | Streamt die Protokolldatei `ctsync.log` |

## <a id="global-options"></a>Globale Optionen

- `--profile <name>`: Verwendet das benannte Profil aus `~/.ctsync/config.toml`.
- `--verbose`: Gibt Debug-Ausgabe aus. Kombinieren Sie diese Option mit `--no-color`, wenn Sie die Ausgabe in eine Datei umleiten.
- `--json`: Gibt maschinenlesbare Ausgabe aus.

## Exit-Codes

| Code | Bedeutung |
|---|---|
| 0 | Erfolg |
| 1 | Allgemeiner Fehler |
| 3 | Nicht angemeldet |
| 7 | Der Ordner ist durch einen anderen Prozess gesperrt |

## <a id="examples"></a>Beispiele

Halten Sie die Synchronisierung während einer Präsentation an:

```bash
ctsync pause --minutes 60
```

Überprüfen Sie den Status eines einzelnen Ordners, und speichern Sie ihn als JSON:

```bash
ctsync status --json > status.json
```
