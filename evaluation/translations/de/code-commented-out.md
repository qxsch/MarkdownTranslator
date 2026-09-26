# <a id="debugging-notes"></a>Debugging-Hinweise

Einige Kommentare enthalten Code, der deaktiviert ist. Er muss exakt unverändert bleiben.

```python
def sync(folder):
    # print(folder.files)
    # folder.refresh(force=True)
    # Nur aktualisieren, wenn sich der Ordner seit der letzten Ausführung geändert hat
    if folder.changed:
        folder.refresh()
    # return None
    return folder.state
```

```javascript
function start() {
  // console.log('starting', config);
  // await client.reset();
  // Watcher nach dem ersten vollständigen Scan starten
  watcher.start();
  // if (debug) { dumpState(); }
}
```

```bash
# export CTSYNC_DEBUG=1
# ctsync reset --force Status jedes
# Ordners anzeigen
ctsync status
```

```csharp
// var client = new SyncClient(uri);
// client.Dispose();
// Gemeinsam genutzten Client wiederverwenden, anstatt einen neuen zu erstellen
var client = SyncClientFactory.Shared;
```

```ts
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const raw: any = JSON.parse(text);
// @ts-expect-error: the SDK types are out of date
client.legacyUpload(raw);
```
