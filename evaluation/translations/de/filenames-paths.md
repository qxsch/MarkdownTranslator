# <a id="files-and-folders"></a>Dateien und Ordner

Der Client speichert seine Einstellungen in `settings.json` und seine Protokolle im Ordner `logs`. Unter Windows lautet der vollständige Pfad `C:\Users\<name>\AppData\Local\ContosoSync\logs\ctsync.log`; unter Linux lautet er `~/.local/share/ctsync/ctsync.log`.

Um den Client zurückzusetzen, löschen Sie settings.json und starten Sie ihn neu. Löschen Sie sync.db nicht, da die Datei die Liste der synchronisierten Dateien enthält.

Öffnen Sie die Datei README.md im Stammverzeichnis des Repositorys, und führen Sie die Schritte aus. Das Skript ./scripts/install.sh installiert die Abhängigkeiten, und deploy.ps1 veröffentlicht den Build im Ordner dist/.

Bilder wie logo.png, banner@2x.jpg und icons/app.ico werden unverändert kopiert.

| Datei | Zweck |
|---|---|
| `config.toml` | Hauptkonfiguration |
| `ignore.txt` | Auszuschließende Muster |
| `Dockerfile` | Container-Build |
| `.env.example` | Vorlage für Umgebungsvariablen |

Protokolldateien, die älter als sieben Tage sind, werden zu `ctsync-2026-08-01.log.gz` komprimiert.
