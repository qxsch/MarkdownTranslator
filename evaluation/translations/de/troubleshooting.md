# <a id="troubleshooting"></a>Problembehandlung

Verwenden Sie diese Seite, wenn die Synchronisierung nicht wie erwartet funktioniert.

## <a id="error-0x8004de40-cant-connect-to-the-server"></a>Fehler 0x8004de40: „Keine Verbindung mit dem Server möglich“

**Ursache:** Ein Proxy oder eine Firewall blockiert die Verbindung.

**Lösung:**

1. Überprüfen Sie, ob `sync.contoso.example` über Port 443 erreichbar ist.
2. Wenn Sie einen Proxy verwenden, legen Sie die Umgebungsvariable `HTTPS_PROXY` fest.
3. Starten Sie den Client mit `ctsync restart` neu.

## <a id="files-stay-in-the-pending-state"></a>Dateien bleiben im Status „Ausstehend“

Dies geschieht in der Regel, wenn eine Datei in einem anderen Programm geöffnet ist. Schließen Sie das Programm, und warten Sie dann einige Minuten. Wenn die Datei weiterhin ausstehend ist, überprüfen Sie das Protokoll:

```powershell
Get-Content "$env:LOCALAPPDATA\ContosoSync\logs\ctsync.log" -Tail 50
```

## <a id="sync-is-slow"></a>Die Synchronisierung ist langsam

- Große Dateien werden in Blöcken von 4 MB hochgeladen. Eine 10-GB-Datei braucht eine Weile.
- Antivirensoftware kann jede Datei zweimal scannen. Schließen Sie den Synchronisierungsordner von der Echtzeitüberprüfung aus.
- Überprüfen Sie die Bandbreitenbegrenzung unter **Einstellungen** > **Netzwerk**.

## <a id="the-app-crashes-at-startup"></a>Die App stürzt beim Start ab

Löschen Sie die Einstellungsdatei `settings.json` in `%APPDATA%\ContosoSync`, und starten Sie die App erneut. Ihre Dateien sind nicht betroffen.

## <a id="still-stuck"></a>Kommen Sie immer noch nicht weiter?

Sammeln Sie Diagnosedaten mit `ctsync diag --zip`, und fügen Sie die Datei `diag.zip` einer Supportanfrage bei.
