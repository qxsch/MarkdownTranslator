# <a id="migration-tasks"></a>Migrationsaufgaben

## <a id="before-the-migration"></a>Vor der Migration

- [x] Liste der Freigaben exportieren
- [x] Alle Benutzer per E-Mail informieren
- [ ] Freigaben entfernen, die im letzten Jahr niemand verwendet hat
- [ ] Prüfen, ob jede Freigabe einen Besitzer hat

## <a id="during-the-migration"></a>Während der Migration

- [ ] Änderungen auf dem alten Server einfrieren
- [ ] `ctsync migrate --source \\oldserver\shares` ausführen
  - [ ] Fortschritt im Admin Center überwachen
  - [ ] Alle Dateien notieren, bei denen Fehler auftreten
- [ ] Dateianzahlen vergleichen

## <a id="after-the-migration"></a>Nach der Migration

1. [ ] Den alten Server nach zwei Wochen ausschalten
2. [ ] Migrationsprotokoll `migrate.log` archivieren
3. [x] Feiern
