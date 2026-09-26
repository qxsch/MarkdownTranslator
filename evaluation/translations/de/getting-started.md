# <a id="getting-started-with-contoso-sync"></a>Erste Schritte mit Contoso Sync

Contoso Sync hält einen lokalen Ordner und eine Cloud-Freigabe synchron. Für diesen Leitfaden benötigen Sie etwa zehn Minuten.

## <a id="prerequisites"></a>Voraussetzungen

Stellen Sie vor Beginn sicher, dass Sie über Folgendes verfügen:

- Ein Contoso-Konto mit der Rolle **Mitwirkender**
- Windows 11 oder macOS 14 oder höher
- Mindestens 2 GB freien Speicherplatz

## <a id="install-the-client"></a>Client installieren

1. Laden Sie `ContosoSync-Setup.exe` von der [Download-Seite](https://contoso.example/download) herunter.
2. Führen Sie das Installationsprogramm aus, und wählen Sie **Weiter** aus.
3. Wenn Sie zur Eingabe eines Ordners aufgefordert werden, behalten Sie den Standardordner `C:\Users\<you>\ContosoSync` bei, oder wählen Sie **Durchsuchen** aus.
4. Wählen Sie **Installieren** und anschließend **Fertig stellen** aus.

> [!TIP]
> Sie können den Client unbeaufsichtigt mit `ContosoSync-Setup.exe /quiet` installieren.

## <a id="sign-in"></a>Anmelden

Öffnen Sie die App über das Startmenü. Geben Sie Ihre geschäftliche E-Mail-Adresse ein, und wählen Sie **Anmelden** aus. Wenn Ihre Organisation mehrstufige Authentifizierung verwendet, genehmigen Sie die Anforderung auf Ihrem Smartphone.

## <a id="choose-what-to-sync"></a>Zu synchronisierende Elemente auswählen

| Option | Standard | Beschreibung |
|---|---|---|
| Dateien bei Bedarf | Ein | Dateien werden erst heruntergeladen, wenn Sie sie öffnen |
| Bei getakteten Verbindungen synchronisieren | Aus | Hält die Synchronisierung bei mobilen Hotspots an |
| Aufbewahren | 30 Tage | Wie lange gelöschte Dateien im Papierkorb verbleiben |

## <a id="next-steps"></a>Nächste Schritte

- Für Ihr Team [einen Ordner freigeben](#choose-what-to-sync)
- Lesen Sie den [Leitfaden zur Problembehandlung](troubleshooting.md)
