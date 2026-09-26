---
title: Créer un rapport de synchronisation avec le SDK Python
description: Combinez le SDK, une tâche planifiée et une petite formule pour générer un rapport sur la croissance du stockage.
---

# <a id="build-a-sync-report-with-the-python-sdk"></a>Créer un rapport de synchronisation avec le SDK Python

> [!NOTE]
> Cet exemple nécessite la version 3.2 ou ultérieure du package `contoso-sync`.

Le taux de croissance $g$ entre deux jours est $g = \frac{b_t - b_{t-1}}{b_{t-1}}$. L’exécution du rapport ne coûte rien, mais le stockage au-delà du quota coûte 0,02 $ par Go.

## <a id="write-the-script"></a>Écrire le script

```python
from contoso_sync import SyncClient


def daily_growth(client, share):
    """Renvoie la croissance du stockage d’un partage pour chaque jour.

    Args:
        client (SyncClient): Un client authentifié.
        share (str): Nom du partage.

    Returns:
        list[float] : Un taux de croissance par jour, du plus ancien au plus récent.
    """
    # Récupère les 30 derniers jours ; l’API retourne les plus récents en premier
    rows = client.usage(share, days=30)[::-1]
    return [(b.bytes - a.bytes) / a.bytes for a, b in zip(rows, rows[1:])]
```

## <a id="schedule-it"></a>Planifier son exécution

| Plateforme | Planificateur | Exemple |
|---|---|---|
| Windows | Planificateur de tâches | Exécuter `python report.py` chaque jour à 06 h 00 |
| Linux | cron | `0 6 * * * python3 /opt/report.py` |

:::tip Faites simple
Commencez par une exécution quotidienne. Les exécutions horaires apportent rarement une valeur ajoutée pour les tendances de stockage.
:::

Consultez [Écrire le script](#write-the-script) si les chiffres semblent incorrects.
