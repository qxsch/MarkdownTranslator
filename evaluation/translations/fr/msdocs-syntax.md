---
title: Connecter un compte de stockage
description: Découvrez comment connecter un compte de stockage existant à Contoso Sync.
ms.date: 08/14/2026
ms.topic: how-to
author: contoso-docs
ms.author: docsteam
ms.service: contoso-sync
---

# <a id="connect-a-storage-account"></a>Connecter un compte de stockage

[!INCLUDE [prérequis pour tous les guides pratiques](../includes/prereqs.md)]

:::image type="content" source="media/connect-storage/overview.png" alt-text="Diagramme montrant le client, le service de synchronisation et le compte de stockage.":::

## <a id="choose-your-platform"></a>Choisir votre plateforme

::: zone pivot="portal"

1. Connectez-vous au portail.
2. Sélectionnez **Stockage** > **Connecter**.
3. Entrez le nom du compte et sélectionnez **Enregistrer**.

::: zone-end

::: zone pivot="cli"

Exécutez la commande suivante :

```azurecli
ctsync storage connect --account mystorage --container sync
```

::: zone-end

> [!div class="nextstepaction"] >
[Configurer la rétention](retention.md)

[!VIDEO https://learn.example/embed/connect-storage]
