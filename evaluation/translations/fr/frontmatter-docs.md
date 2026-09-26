---
title: Surveiller l’intégrité de la synchronisation
titleSuffix: Documentation de Contoso Sync
description: Utilisez le tableau de bord d’intégrité et les alertes pour détecter les problèmes de synchronisation avant que vos utilisateurs ne les remarquent.
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

# <a id="monitor-sync-health"></a>Surveiller l’intégrité de la synchronisation

Le tableau de bord d’intégrité affiche l’état de synchronisation de tous les appareils de votre organisation.

## <a id="set-up-alerts"></a>Configurer les alertes

1. Dans le centre d’administration, sélectionnez **Intégrité** > **Alertes**.
2. Sélectionnez **Nouvelle règle d’alerte**.
3. Choisissez une condition, par exemple *Plus de 5 % des appareils présentent des erreurs*.
4. Ajoutez un groupe d’actions et sélectionnez **Créer**.

## <a id="query-the-data"></a>Interroger les données

Vous pouvez également interroger les données avec PowerShell :

```powershell
# Obtenir les appareils qui ne se sont pas synchronisés depuis trois jours
Get-CtSyncDevice -LastSyncBefore (Get-Date).AddDays(-3)
```
