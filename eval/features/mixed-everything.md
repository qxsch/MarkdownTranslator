---
title: Build a sync report with the Python SDK
description: Combine the SDK, a scheduled job and a small formula to report storage growth.
---

# Build a sync report with the Python SDK

> [!NOTE]
> This sample needs version 3.2 or later of the `contoso-sync` package.

The growth rate $g$ between two days is $g = \frac{b_t - b_{t-1}}{b_{t-1}}$. The report costs nothing to run, but storage beyond the quota costs $0.02 per GB.

## Write the script

```python
from contoso_sync import SyncClient


def daily_growth(client, share):
    """Return the storage growth of a share for each day.

    Args:
        client (SyncClient): An authenticated client.
        share (str): Name of the share.

    Returns:
        list[float]: One growth rate per day, oldest first.
    """
    # Fetch the last 30 days; the API returns newest first
    rows = client.usage(share, days=30)[::-1]
    return [(b.bytes - a.bytes) / a.bytes for a, b in zip(rows, rows[1:])]
```

## Schedule it

| Platform | Scheduler | Example |
|---|---|---|
| Windows | Task Scheduler | Run `python report.py` daily at 06:00 |
| Linux | cron | `0 6 * * * python3 /opt/report.py` |

:::tip Keep it simple
Start with a daily run. Hourly runs rarely add value for storage trends.
:::

See [Write the script](#write-the-script) if the numbers look wrong.
