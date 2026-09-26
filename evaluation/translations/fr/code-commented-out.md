# <a id="debugging-notes"></a>Notes de débogage

Certains commentaires contiennent du code qui est désactivé. Il doit rester exactement tel quel.

```python
def sync(folder):
    # print(folder.files)
    # folder.refresh(force=True)
    # Actualiser uniquement lorsque le dossier a changé depuis la dernière exécution
    if folder.changed:
        folder.refresh()
    # return None
    return folder.state
```

```javascript
function start() {
  // console.log('starting', config);
  // await client.reset();
  // Démarrer l’observateur après la première analyse complète
  watcher.start();
  // if (debug) { dumpState(); }
}
```

```bash
# export CTSYNC_DEBUG=1
# ctsync reset --force Afficher l’état
# de chaque dossier
ctsync status
```

```csharp
// var client = new SyncClient(uri);
// client.Dispose();
// Réutiliser le client partagé au lieu d’en créer un nouveau
var client = SyncClientFactory.Shared;
```

```ts
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const raw: any = JSON.parse(text);
// @ts-expect-error: the SDK types are out of date
client.legacyUpload(raw);
```
