---
title: Speicherkonto verbinden
description: Erfahren Sie, wie Sie ein vorhandenes Speicherkonto mit Contoso Sync verbinden.
ms.date: 08/14/2026
ms.topic: how-to
author: contoso-docs
ms.author: docsteam
ms.service: contoso-sync
---

# <a id="connect-a-storage-account"></a>Speicherkonto verbinden

[!INCLUDE [Voraussetzungen für alle Anleitungen](../includes/prereqs.md)]

:::image type="content" source="media/connect-storage/overview.png" alt-text="Diagramm, das den Client, den Synchronisierungsdienst und das Speicherkonto zeigt.":::

## <a id="choose-your-platform"></a>Plattform auswählen

::: zone pivot="portal"

1. Melden Sie sich beim Portal an.
2. Wählen Sie **Speicher** > **Verbinden** aus.
3. Geben Sie den Kontonamen ein, und wählen Sie **Speichern** aus.

::: zone-end

::: zone pivot="cli"

Führen Sie den folgenden Befehl aus:

```azurecli
ctsync storage connect --account mystorage --container sync
```

::: zone-end

> [!div class="nextstepaction"] >
[Aufbewahrung
konfigurieren](retention.md)

[!VIDEO https://learn.example/embed/connect-storage]
