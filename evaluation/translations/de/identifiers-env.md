# <a id="environment-and-identifiers"></a>Umgebung und Bezeichner

Legen Sie `CTSYNC_HOME` fest, um den Datenordner zu ändern. Unter Windows lautet der Standardwert `%LOCALAPPDATA%\ContosoSync`; unter Linux lautet er `$HOME/.ctsync`.

Der Client liest den Proxy aus HTTPS_PROXY und ignoriert NO_PROXY-Einträge, die Platzhalterzeichen enthalten.

Verwenden Sie das Cmdlet Get-CtSyncStatus, um den Status über PowerShell zu prüfen, oder rufen Sie die Methode getSyncStatus() aus dem JavaScript-SDK auf. Im Python-SDK heißt sie get_sync_status.

Das Flag --max-retries akzeptiert Werte von 0 bis 10. Die Option maxRetries in `config.toml` bewirkt dasselbe.

Verbindungszeichenfolgen sehen wie `Endpoint=https://sync.contoso.example;SharedAccessKey=<key>` aus. Ersetzen Sie `${TENANT_ID}` durch Ihre Mandanten-ID, z. B. 00000000-0000-0000-0000-000000000000.

Mit Version 3.2.0-beta.1 wurde die Einstellung `upload.parallelism` eingeführt; mit Version v3.2.1 wurde ihr Standardwert auf 8 angehoben.
