# Using the JavaScript SDK

Install the package and create a client:

```javascript
import { SyncClient } from '@contoso/sync';

// Create one client per process and reuse it.
const client = new SyncClient({ endpoint: process.env.CTSYNC_ENDPOINT });

/*
 * Upload a file. The promise resolves when the last block
 * has been confirmed by the server.
 */
await client.upload('report.pdf', { overwrite: true }); // overwrite existing versions
```

TypeScript users get full type information:

```typescript
/**
 * Options for a folder watch.
 * @param path The folder to watch, relative to the sync root.
 * @param recursive Whether subfolders are included.
 * @returns A handle that stops the watch when disposed.
 */
export function watch(path: string, recursive = true): Disposable {
  // TODO: support glob patterns
  return client.watch(path, { recursive });
}

// Retry three times with exponential backoff
const policy: RetryPolicy = { retries: 3, backoff: 'exponential' };
```

React components can use the hook:

```tsx
export function SyncBadge() {
  // Re-renders whenever the sync state changes
  const state = useSyncState();
  return <Badge color={state === 'ok' ? 'green' : 'red'}>{state}</Badge>; {/* label shown to the user */}
}
```
