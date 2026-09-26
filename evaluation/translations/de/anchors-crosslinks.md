# <a id="administrator-guide"></a>Administratorhandbuch

In diesem Handbuch finden Sie viele Links auf Abschnitte innerhalb des Handbuchs. Beginnen Sie mit [Planen des Rollouts](#plan-the-rollout) und lesen Sie anschließend [Lizenzen zuweisen](#assign-licenses).

## <a id="plan-the-rollout"></a>Planen des Rollouts

Entscheiden Sie, welche Gruppen den Client zuerst erhalten. Empfehlungen finden Sie unter [Pilotgruppen](#pilot-groups). Falls etwas schiefgeht, lesen Sie den Abschnitt [Rollback durchführen](#roll-back).

### <a id="pilot-groups"></a>Pilotgruppen

Beginnen Sie mit einer Gruppe von 20 bis 50 Benutzern. Deren Feedback hilft Ihnen, die [Standardeinstellungen](#default-settings) abzustimmen.

## <a id="assign-licenses"></a>Lizenzen zuweisen

Lizenzen werden pro Benutzer zugewiesen. Die gruppenbasierte Zuweisung wird unter [Zuweisung automatisieren](#automate-assignment) beschrieben.

### <a id="automate-assignment"></a>Zuweisung automatisieren

Verwenden Sie die Admin-API, um Lizenzen aus Ihrem HR-System zuzuweisen. Die API wird in der [Fehlerreferenz](api-errors-table.md#handling-conflicts) beschrieben.

## <a id="default-settings"></a>Standardeinstellungen

Verteilen Sie Standardeinstellungen mit Ihrem Geräteverwaltungstool. Einstellungen, die Benutzer nicht ändern können, sind als **gesperrt** gekennzeichnet.

## <a id="roll-back"></a>Rollback durchführen

Um ein Rollback durchzuführen, deinstallieren Sie den Client und entfernen Sie die Richtlinie. Dateien verbleiben in der Cloud. Kehren Sie zum [Anfang](#administrator-guide) zurück.
