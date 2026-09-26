# <a id="getting-started-with-contoso-sync"></a>Erste Schritte mit Contoso Sync

Contoso Sync hält einen lokalen Ordner und eine Cloud-Freigabe synchron. Für diesen Leitfaden benötigen Sie etwa zehn Minuten.

## <a id="prerequisites"></a>Voraussetzungen

Bevor Sie beginnen, stellen Sie sicher, dass Folgendes vorhanden ist:

- Ein Contoso-Konto mit der Rolle **Mitwirkender**
- Windows 11 oder macOS 14 oder höher
- Mindestens 2 GB freier Speicherplatz

## <a id="install-the-client"></a>Client installieren

1. Laden Sie `ContosoSync-Setup.exe` von der [Download-Seite](https://contoso.example/download) herunter.
2. Führen Sie das Installationsprogramm aus und wählen Sie **Weiter** aus.
3. Wenn Sie zur Angabe eines Ordners aufgefordert werden, behalten Sie den Standardordner `C:\Users\<you>\ContosoSync` bei oder wählen Sie **Durchsuchen** aus.
4. Wählen Sie **Installieren** und anschließend **Fertig stellen** aus.

> [!TIP]
> Für eine unbeaufsichtigte Installation des Clients verwenden Sie `ContosoSync-Setup.exe /quiet`.

## <a id="sign-in"></a>Anmelden

Öffnen Sie die App über das Startmenü. Geben Sie Ihre geschäftliche E-Mail-Adresse ein und wählen Sie **Anmelden** aus. Wenn Ihre Organisation mehrstufige Authentifizierung verwendet, genehmigen Sie die Anforderung auf Ihrem Telefon.

## <a id="choose-what-to-sync"></a>Auswählen, was synchronisiert werden soll

| Option | Standard | Beschreibung |
|---|---|---|
| Dateien bei Bedarf | Ein | Dateien werden erst heruntergeladen, wenn Sie sie öffnen |
| In getakteten Netzwerken synchronisieren | Aus | Hält die Synchronisierung bei mobilen Hotspots an |
| Aufbewahren | 30 Tage | Wie lange gelöschte Dateien im Papierkorb verbleiben |

## <a id="next-steps"></a>Nächste Schritte

- Für Ihr Team [einen Ordner freigeben](#choose-what-to-sync)
- Lesen Sie den [Leitfaden zur Problembehandlung](troubleshooting.md)
