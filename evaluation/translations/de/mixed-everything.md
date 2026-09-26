---
title: Erstellen eines Sync-Berichts mit dem Python-SDK
description: Kombinieren Sie das SDK, einen geplanten Auftrag und eine kleine Formel, um Speicherwachstum zu melden.
---

# <a id="build-a-sync-report-with-the-python-sdk"></a>Erstellen eines Sync-Berichts mit dem Python-SDK

> [!NOTE]
> Für dieses Beispiel ist Version 3.2 oder höher des `contoso-sync`-Pakets erforderlich.

Die Wachstumsrate $g$ zwischen zwei Tagen beträgt $g = \frac{b_t - b_{t-1}}{b_{t-1}}$. Die Ausführung des Berichts ist kostenlos, aber Speicherplatz über das Kontingent hinaus kostet 0,02 $ pro GB.

## <a id="write-the-script"></a>Skript schreiben

```python
from contoso_sync import SyncClient


def daily_growth(client, share):
    """Gibt das Speicherwachstum einer Freigabe für jeden Tag zurück.

    Args:
        client (SyncClient): Ein authentifizierter Client.
        share (str): Name der Freigabe.

    Returns:
        list[float]: Eine Wachstumsrate pro Tag, älteste zuerst.
    """
    # Letzte 30 Tage abrufen; die API gibt die neuesten zuerst zurück
    rows = client.usage(share, days=30)[::-1]
    return [(b.bytes - a.bytes) / a.bytes for a, b in zip(rows, rows[1:])]
```

## <a id="schedule-it"></a>Ausführung planen

| Plattform | Scheduler | Beispiel |
|---|---|---|
| Windows | Task Scheduler | Führen Sie `python report.py` täglich um 06:00 Uhr aus |
| Linux | cron | `0 6 * * * python3 /opt/report.py` |

:::tip Halten Sie es einfach
Beginnen Sie mit einer täglichen Ausführung. Stündliche Ausführungen bieten bei Speichertrends selten einen Mehrwert.
:::

Weitere Informationen finden Sie unter [Skript schreiben](#write-the-script), wenn die Zahlen falsch aussehen.
