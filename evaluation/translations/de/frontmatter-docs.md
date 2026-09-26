---
title: Synchronisierungsintegrität überwachen
titleSuffix: Contoso Sync-Dokumentation
description: Verwenden Sie das Integritätsdashboard und Warnungen, um Synchronisierungsprobleme zu finden, bevor Ihre Benutzer sie bemerken.
ms.date: 08/14/2026
ms.topic: how-to
ms.service: contoso-sync
ms.custom: devx-track-powershell
author: contoso-docs
ms.author: docsteam
ms.reviewer: sre-team
ms.collection: [admin, monitoring]
zone_pivot_groups: operating-systems
---

# <a id="monitor-sync-health"></a>Synchronisierungsintegrität überwachen

Das Integritätsdashboard zeigt den Synchronisierungsstatus aller Geräte in Ihrer Organisation an.

## <a id="set-up-alerts"></a>Warnungen einrichten

1. Wählen Sie im Admin Center **Integrität** > **Warnungen** aus.
2. Wählen Sie **Neue Warnungsregel** aus.
3. Wählen Sie eine Bedingung aus, z. B. *Bei mehr als 5 % der Geräte treten Fehler auf*.
4. Fügen Sie eine Aktionsgruppe hinzu, und wählen Sie **Erstellen** aus.

## <a id="query-the-data"></a>Daten abfragen

Sie können die Daten auch mit PowerShell abfragen:

```powershell
# Geräte abrufen, die seit drei Tagen nicht synchronisiert wurden
Get-CtSyncDevice -LastSyncBefore (Get-Date).AddDays(-3)
```
