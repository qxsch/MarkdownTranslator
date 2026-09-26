# <a id="using-the-javascript-sdk"></a>Verwenden des JavaScript-SDK

Installieren Sie das Paket, und erstellen Sie einen Client:

```javascript
import { SyncClient } from '@contoso/sync';

// Erstellen Sie einen Client pro Prozess, und verwenden Sie ihn wieder.
const client = new SyncClient({ endpoint: process.env.CTSYNC_ENDPOINT });

/*
 * Laden Sie eine Datei hoch. Das Promise wird aufgelöst,
 * wenn der letzte Block vom Server bestätigt wurde.
 */
await client.upload('report.pdf', { overwrite: true }); // vorhandene Versionen überschreiben
```

TypeScript-Benutzer erhalten vollständige Typinformationen:

```typescript
/**
 * Optionen für eine Ordnerüberwachung.
 * @param path Der zu überwachende Ordner, relativ zum Synchronisierungsstamm.
 * @param recursive Ob Unterordner eingeschlossen werden.
 * @returns Ein Handle, das die Überwachung beendet, wenn es freigegeben wird.
 */
export function watch(path: string, recursive = true): Disposable {
  // TODO: Glob-Muster unterstützen
  return client.watch(path, { recursive });
}

// Drei Wiederholungsversuche mit exponentiellem Backoff
const policy: RetryPolicy = { retries: 3, backoff: 'exponential' };
```

React-Komponenten können den Hook verwenden:

```tsx
export function SyncBadge() {
  // Wird bei jeder Änderung des Synchronisierungsstatus neu gerendert
  const state = useSyncState();
  return <Badge color={state === 'ok' ? 'green' : 'red'}>{state}</Badge>; {/* dem Benutzer angezeigte Beschriftung */}
}
```
