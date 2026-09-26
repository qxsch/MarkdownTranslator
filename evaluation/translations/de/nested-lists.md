# <a id="deployment-checklist"></a>Checkliste für die Bereitstellung

Führen Sie vor einer Produktionsbereitstellung die folgenden Aufgaben aus:

- Infrastruktur
  - Überprüfen Sie das Kontingent in der Zielregion:
    - CPU-Kerne
    - Öffentliche IP-Adressen
    - Speicherkonten
  - Vergewissern Sie sich, dass die Netzwerkregeln Folgendes zulassen:
    1. Ausgehenden Datenverkehr über Port 443
    2. Eingehenden Datenverkehr ausschließlich vom Load Balancer
- Anwendung
  - Führen Sie die Testsuite aus. Alle folgenden Tests müssen erfolgreich sein:
    - Unit-Tests
    - Integrationstests, einschließlich der langsamen Tests
  - Taggen Sie das Release, z. B. `v3.2.0`
- Beteiligte
  - Informieren Sie das Support-Team mindestens einen Tag im Voraus
  - Benennen Sie einen Bereitschaftstechniker, der Folgendes ausführen kann:
    - Ein Rollback für das Release ausführen
    - Den Kunden kontaktieren

Überprüfen Sie nach der Bereitstellung die folgenden Punkte:

1. Der Integritätsendpunkt `/healthz` gibt `200` zurück.
2. Fehlerraten bleiben 30 Minuten lang unter der Baseline.
3. Es werden keine Warnungen ausgelöst.
   - Wenn eine Warnung ausgelöst wird, befolgen Sie das Runbook.
   - Wenn das Runbook nicht hilft, führen Sie ein Rollback aus.
