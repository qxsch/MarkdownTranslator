# <a id="tutorial-automate-backups-with-contoso-sync"></a>Tutorial: Sicherungen mit Contoso Sync automatisieren

In diesem Tutorial richten Sie eine nächtliche Sicherung eines Projektordners ein, testen eine Wiederherstellung und werden benachrichtigt, wenn ein Fehler auftritt.

## <a id="part-1-prepare-the-folder"></a>Teil 1: Ordner vorbereiten

Erstellen Sie in Ihrem Home-Verzeichnis einen Ordner namens `projects`. Verschieben Sie die Dateien, die Sie sichern möchten, in diesen Ordner. Vermeiden Sie Ordner, die Build-Ausgabe enthalten, z. B. `node_modules` oder `bin`, da sie sich häufig ändern und viel Speicherplatz belegen.

Fügen Sie als Nächstes eine Datei namens `.ctsyncignore` hinzu, um Muster auszuschließen:

```text
node_modules/
bin/
*.tmp
```

## <a id="part-2-create-the-backup-job"></a>Teil 2: Sicherungsauftrag erstellen

1. Öffnen Sie **Einstellungen** > **Sicherungen**.
2. Wählen Sie **Neuer Auftrag** aus.
3. Geben Sie einen Namen ein, z. B. *Nächtliche Projekte*.
4. Wählen Sie unter **Zeitplan** die Option **Täglich** aus, und legen Sie die Uhrzeit auf 02:00 Uhr fest.
5. Wählen Sie unter **Beibehalten** aus, wie viele Versionen beibehalten werden sollen. Sieben ist ein guter Anfang.
6. Wählen Sie **Erstellen** aus.

Der Auftrag wird in der Liste mit dem Status **Geplant** angezeigt.

## <a id="part-3-test-a-restore"></a>Teil 3: Wiederherstellung testen

Eine Sicherung ist nur nützlich, wenn Sie daraus wiederherstellen können. Löschen Sie eine Testdatei, und gehen Sie dann wie folgt vor:

1. Wählen Sie den Auftrag und dann **Wiederherstellen** aus.
2. Wählen Sie die Version von gestern aus.
3. Wählen Sie **Am ursprünglichen Speicherort wiederherstellen** aus.

Die Datei wird innerhalb weniger Sekunden wieder angezeigt.

## <a id="part-4-get-notified"></a>Teil 4: Benachrichtigungen erhalten

Aktivieren Sie unter **Benachrichtigungen** die Option **Mir eine E-Mail senden, wenn ein Auftrag fehlschlägt**. Sie können Ereignisse auch an einen Webhook senden:

```json
{
  "webhook": "https://hooks.example.com/backup",
  "events": ["job.failed", "job.succeeded"]
}
```

## <a id="clean-up"></a>Bereinigen

Wenn Sie den Auftrag nicht behalten möchten, wählen Sie ihn und dann **Löschen** aus. Durch das Löschen des Auftrags werden vorhandene Sicherungen nicht gelöscht; sie laufen entsprechend der Aufbewahrungseinstellung ab.

## <a id="summary"></a>Zusammenfassung

Sie haben eine geplante Sicherung erstellt, eine Wiederherstellung getestet und Benachrichtigungen eingerichtet. Fahren Sie mit [Ordner für Ihr Team freigeben](share.md) fort.
