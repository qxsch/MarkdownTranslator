---
title: Connect a storage account
description: Learn how to connect an existing storage account to Contoso Sync.
ms.date: 08/14/2026
ms.topic: how-to
author: contoso-docs
ms.author: docsteam
ms.service: contoso-sync
---

# Connect a storage account

[!INCLUDE [prerequisites for all how-to guides](../includes/prereqs.md)]

:::image type="content" source="media/connect-storage/overview.png" alt-text="Diagram that shows the client, the sync service and the storage account.":::

## Choose your platform

::: zone pivot="portal"

1. Sign in to the portal.
2. Select **Storage** > **Connect**.
3. Enter the account name and select **Save**.

::: zone-end

::: zone pivot="cli"

Run the following command:

```azurecli
ctsync storage connect --account mystorage --container sync
```

::: zone-end

> [!div class="nextstepaction"]
> [Configure retention](retention.md)

[!VIDEO https://learn.example/embed/connect-storage]
