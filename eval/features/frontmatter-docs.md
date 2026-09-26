---
title: Monitor sync health
titleSuffix: Contoso Sync documentation
description: Use the health dashboard and alerts to find sync problems before your users notice them.
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

# Monitor sync health

The health dashboard shows the sync state of all devices in your organization.

## Set up alerts

1. In the admin center, select **Health** > **Alerts**.
2. Select **New alert rule**.
3. Choose a condition, for example *More than 5% of devices have errors*.
4. Add an action group and select **Create**.

## Query the data

You can also query the data with PowerShell:

```powershell
# Get devices that haven't synced for three days
Get-CtSyncDevice -LastSyncBefore (Get-Date).AddDays(-3)
```
