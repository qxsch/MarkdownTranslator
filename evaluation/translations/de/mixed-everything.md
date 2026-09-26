---
title: Erstellen eines Synchronisierungsberichts mit dem Python-SDK
description: Kombinieren Sie das SDK, einen geplanten Auftrag und eine kleine Formel, um über den Speicherzuwachs zu berichten.
---

# <a id="build-a-sync-report-with-the-python-sdk"></a>Erstellen eines Synchronisierungsberichts mit dem Python-SDK

> [!NOTE]
> Für dieses Beispiel ist Version 3.2 oder höher des `contoso-sync`-Pakets erforderlich.

Die Wachstumsrate $g$ zwischen zwei Tagen beträgt $g = \frac{b_t - b_{t-1}}{b_{t-1}}$. Die Ausführung des Berichts verursacht keine Kosten, aber Speicher über das Kontingent hinaus kostet 0,02 $ pro GB.

## <a id="write-the-script"></a>Schreiben des Skripts

```python
from contoso_sync import SyncClient


def daily_growth(client, share):
    """Gibt den Speicherzuwachs einer Freigabe für jeden Tag zurück.

    Args:
        client (SyncClient): Ein authentifizierter Client.
        share (str): Name der Freigabe.

    Returns:
        list[float]: Eine Wachstumsrate pro Tag, älteste zuerst.
    """
    # Die letzten 30 Tage abrufen; die API gibt die neuesten zuerst zurück
    rows = client.usage(share, days=30)[::-1]
    return [(b.bytes - a.bytes) / a.bytes for a, b in zip(rows, rows[1:])]
```

## <a id="schedule-it"></a>Planen der Ausführung

| Plattform | Zeitplanungstool | Beispiel |
|---|---|---|
| Windows | Aufgabenplanung | Führen Sie `python report.py` täglich um 06:00 Uhr aus |
| Linux | cron | `0 6 * * * python3 /opt/report.py` |

:::tip Einfach halten
Beginnen Sie mit einer täglichen Ausführung. Stündliche Ausführungen bieten bei Speichertrends selten einen Mehrwert.
:::

Wenn die Zahlen falsch aussehen, lesen Sie den Abschnitt [Schreiben des Skripts](#write-the-script).
