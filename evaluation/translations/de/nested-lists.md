# <a id="deployment-checklist"></a>Bereitstellungscheckliste

Führen Sie vor einer Produktionsbereitstellung die folgenden Aufgaben aus:

- Infrastruktur
  - Prüfen Sie das Kontingent in der Zielregion:
    - CPU-Kerne
    - Öffentliche IP-Adressen
    - Speicherkonten
  - Bestätigen Sie, dass die Netzwerkregeln Folgendes zulassen:
    1. Ausgehender Datenverkehr über Port 443
    2. Eingehender Datenverkehr ausschließlich vom Lastenausgleich
- Anwendung
  - Führen Sie die Testsuite aus. Alle folgenden Tests müssen erfolgreich sein:
    - Komponententests
    - Integrationstests, einschließlich der langsamen
  - Versehen Sie das Release mit einem Tag, zum Beispiel `v3.2.0`
- Personen
  - Informieren Sie das Supportteam mindestens einen Tag im Voraus
  - Benennen Sie einen On-Call-Engineer, der Folgendes tun kann:
    - Ein Rollback des Releases durchführen
    - Den Kunden kontaktieren

Prüfen Sie nach der Bereitstellung die folgenden Punkte:

1. Der Integritätsendpunkt `/healthz` gibt `200` zurück.
2. Die Fehlerraten bleiben 30 Minuten lang unter der Baseline.
3. Es werden keine Warnungen ausgelöst.
   - Wenn eine Warnung ausgelöst wird, folgen Sie dem Runbook.
   - Wenn das Runbook nicht hilft, führen Sie ein Rollback durch.
