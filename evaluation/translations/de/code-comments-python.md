# <a id="using-the-python-sdk"></a>Verwenden des Python-SDK

```python
import os
from contoso_sync import SyncClient

# Endpunkt aus der Umgebung lesen, damit das Skript in jeder Phase funktioniert
client = SyncClient(endpoint=os.environ["CTSYNC_ENDPOINT"])

# Jede CSV-Datei im aktuellen Ordner hochladen
for name in os.listdir("."):
    if name.endswith(".csv"):  # Alles andere überspringen
        client.upload(name)

# Warten, bis der Server alle Uploads verarbeitet
# hat. Bei großen Dateien kann dies einige Minuten
# dauern.
client.wait()
```

Behandeln Sie Fehler explizit:

```python
try:
    client.upload("big.iso")
except QuotaExceededError:
    # Der Speicher ist voll: Benutzer informieren, statt es erneut zu versuchen
    print("Storage quota exceeded")
```
