# <a id="windows-line-endings"></a>Windows-Zeilenenden

Diese Datei verwendet CRLF-Zeilenenden. Sie müssen bei der Übersetzung Byte für Byte erhalten bleiben.

## <a id="steps"></a>Schritte

1. Öffnen Sie die **Systemsteuerung**.
2. Wählen Sie **Programme** > **Programm deinstallieren** aus.
3. Wählen Sie *Contoso Sync* und dann **Deinstallieren** aus.

```powershell
# Verbleibenden Datenordner entfernen
Remove-Item -Recurse "$env:LOCALAPPDATA\ContosoSync"
```

> [!NOTE]
> Ihre Dateien in der Cloud werden nicht gelöscht.
