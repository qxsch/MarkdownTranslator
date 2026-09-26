# <a id="migration-tasks"></a>Migrationsaufgaben

## <a id="before-the-migration"></a>Vor der Migration

- [x] Liste der Freigaben exportieren
- [x] Alle Benutzer per E-Mail informieren
- [ ] Freigaben entfernen, die innerhalb des letzten Jahres niemand verwendet hat
- [ ] Prüfen, ob jede Freigabe einen Besitzer hat

## <a id="during-the-migration"></a>Während der Migration

- [ ] Änderungen auf dem alten Server einfrieren
- [ ] `ctsync migrate --source \\oldserver\shares` ausführen
  - [ ] Den Fortschritt im Administrationscenter überwachen
  - [ ] Alle fehlgeschlagenen Dateien notieren
- [ ] Dateianzahlen vergleichen

## <a id="after-the-migration"></a>Nach der Migration

1. [ ] Den alten Server nach zwei Wochen ausschalten
2. [ ] Das Migrationsprotokoll `migrate.log` archivieren
3. [x] Feiern
