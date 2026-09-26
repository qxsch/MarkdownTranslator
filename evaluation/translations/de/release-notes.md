# <a id="release-notes"></a>Versionshinweise

## Version 3.2.0 (2026-08-14)

### <a id="new-features"></a>Neue Funktionen

- **Selektive Synchronisierung von Unterordnern.** Sie können jetzt einzelne Unterordner statt ganzer Freigaben ausschließen.
- Ein neues `--dry-run`-Flag für `ctsync reset` zeigt an, was gelöscht würde.
- Der dunkle Modus richtet sich nach der Systemeinstellung.

### <a id="fixed-issues"></a>Behobene Probleme

- Ein Absturz wurde behoben, der auftrat, wenn ein Dateiname ein Emoji enthielt.
- Das Symbol im Infobereich flackert bei großen Uploads nicht mehr.
- Wenn ein Ordner während der Synchronisierung umbenannt wurde, wurde ein Duplikat erstellt. Dies wurde behoben.

### <a id="known-issues"></a>Bekannte Probleme

- Unter macOS 14.1 werden Benachrichtigungen möglicherweise zweimal angezeigt. Aktualisieren Sie auf 14.2, um dieses Problem zu beheben.

## Version 3.1.4 (2026-06-02)

- Sicherheitsupdate für CVE-2026-1234. Aktualisieren Sie so bald wie möglich.
- Die Upload-Geschwindigkeit bei langsamen Verbindungen wurde um bis zu 30 % verbessert.

## Version 3.1.0 (2026-04-20)

### <a id="breaking-changes"></a>Nicht abwärtskompatible Änderungen

- Die Konfigurationsdatei wurde von `config.ini` nach `config.toml` verschoben. Die App migriert sie beim ersten Start automatisch.
- Die Unterstützung für Windows 10 Version 1809 wurde eingestellt.
