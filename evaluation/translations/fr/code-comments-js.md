# <a id="using-the-javascript-sdk"></a>Utilisation du SDK JavaScript

Installez le package et créez un client :

```javascript
import { SyncClient } from '@contoso/sync';

// Créez un client par processus et réutilisez-le.
const client = new SyncClient({ endpoint: process.env.CTSYNC_ENDPOINT });

/*
 * Chargez un fichier. La promesse est résolue lorsque le
 * dernier bloc a été confirmé par le serveur.
 */
await client.upload('report.pdf', { overwrite: true }); // remplacer les versions existantes
```

Les utilisateurs de TypeScript bénéficient d’informations complètes sur les types :

```typescript
/**
 * Options pour la surveillance d’un dossier.
 * @param path Le dossier à surveiller, relatif à la racine de synchronisation.
 * @param recursive Indique si les sous-dossiers sont inclus.
 * @returns Un descripteur qui arrête la surveillance lorsqu’il est libéré.
 */
export function watch(path: string, recursive = true): Disposable {
  // TODO: prendre en charge les modèles glob
  return client.watch(path, { recursive });
}

// Réessayer trois fois avec une temporisation exponentielle
const policy: RetryPolicy = { retries: 3, backoff: 'exponential' };
```

Les composants React peuvent utiliser le hook :

```tsx
export function SyncBadge() {
  // Effectue un nouveau rendu chaque fois que l’état de synchronisation change
  const state = useSyncState();
  return <Badge color={state === 'ok' ? 'green' : 'red'}>{state}</Badge>; {/* libellé affiché à l’utilisateur */}
}
```
