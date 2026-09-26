# Using the Python SDK

```python
import os
from contoso_sync import SyncClient

# Read the endpoint from the environment so that the script works in every stage
client = SyncClient(endpoint=os.environ["CTSYNC_ENDPOINT"])

# Upload every CSV file in the current folder
for name in os.listdir("."):
    if name.endswith(".csv"):  # skip everything else
        client.upload(name)

# Wait until the server has processed all uploads.
# This can take a few minutes for large files.
client.wait()
```

Handle errors explicitly:

```python
try:
    client.upload("big.iso")
except QuotaExceededError:
    # The storage is full: tell the user instead of retrying
    print("Storage quota exceeded")
```
