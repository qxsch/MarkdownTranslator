# <a id="administrator-guide"></a>Administratorhandbuch

In diesem Handbuch wird häufig auf eigene Abschnitte verlinkt. Beginnen Sie mit [Rollout planen](#plan-the-rollout), und lesen Sie anschließend [Lizenzen zuweisen](#assign-licenses).

## <a id="plan-the-rollout"></a>Rollout planen

Entscheiden Sie, welche Gruppen den Client zuerst erhalten. Empfehlungen finden Sie unter [Pilotgruppen](#pilot-groups); Informationen für den Fall, dass etwas schiefgeht, finden Sie unter [Rollback ausführen](#roll-back).

### <a id="pilot-groups"></a>Pilotgruppen

Beginnen Sie mit einer Gruppe von 20 bis 50 Benutzern. Deren Feedback hilft Ihnen, die [Standardeinstellungen](#default-settings) abzustimmen.

## <a id="assign-licenses"></a>Lizenzen zuweisen

Lizenzen werden pro Benutzer zugewiesen. Die gruppenbasierte Zuweisung wird unter [Zuweisung automatisieren](#automate-assignment) beschrieben.

### <a id="automate-assignment"></a>Zuweisung automatisieren

Verwenden Sie die Admin-API, um Lizenzen aus Ihrem HR-System zuzuweisen. Die API ist in der [Fehlerreferenz](api-errors-table.md#handling-conflicts) beschrieben.

## <a id="default-settings"></a>Standardeinstellungen

Stellen Sie Standardeinstellungen über Ihr Geräteverwaltungstool bereit. Einstellungen, die Benutzer nicht ändern können, sind als **gesperrt** gekennzeichnet.

## <a id="roll-back"></a>Rollback ausführen

Um ein Rollback auszuführen, deinstallieren Sie den Client, und entfernen Sie die Richtlinie. Dateien verbleiben in der Cloud. Kehren Sie zum [Anfang](#administrator-guide) zurück.
