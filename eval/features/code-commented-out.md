# Debugging notes

Some comments contain code that is switched off. It must stay exactly as it is.

```python
def sync(folder):
    # print(folder.files)
    # folder.refresh(force=True)
    # Refresh only when the folder changed since the last run
    if folder.changed:
        folder.refresh()
    # return None
    return folder.state
```

```javascript
function start() {
  // console.log('starting', config);
  // await client.reset();
  // Start the watcher after the first full scan
  watcher.start();
  // if (debug) { dumpState(); }
}
```

```bash
# export CTSYNC_DEBUG=1
# ctsync reset --force
# Show the status of every folder
ctsync status
```

```csharp
// var client = new SyncClient(uri);
// client.Dispose();
// Reuse the shared client instead of creating a new one
var client = SyncClientFactory.Shared;
```

```ts
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const raw: any = JSON.parse(text);
// @ts-expect-error: the SDK types are out of date
client.legacyUpload(raw);
```
