# <a id="infrastructure-and-service-code"></a>Code d’infrastructure et de service

Go :

```go
package main

// Config contient les paramètres lus au démarrage.
type Config struct {
	Endpoint string // URL du service de synchronisation
	Workers  int    // nombre de chargements parallèles
}
```

Rust :

```rust
/// Calcule le hachage d’un bloc.
fn hash_block(data: &[u8]) -> [u8; 32] {
    // SHA-256 est assez rapide pour des blocs de 4 Mo
    sha256(data)
}
```

Java :

```java
/**
 * Surveille les modifications et les place en file d’attente pour chargement.
 */
public class ChangeListener {
    // Les modifications sont regroupées par lots toutes les 500 ms
    private static final int BATCH_MS = 500;
}
```

SQL :

```sql
-- Trouver les appareils qui ne se sont pas synchronisés depuis une semaine
SELECT device_id, last_sync
FROM devices
WHERE last_sync < DATEADD(day, -7, GETUTCDATE()); /* UTC volontairement */
```

YAML:

```yaml
# Paramètres de l’agent de synchronisation
agent:
  workers: 8 # un par cœur de processeur est un bon point de départ
  log_level: info
```

Bicep:

```bicep
// Compte de stockage pour les données de synchronisation
resource storage 'Microsoft.Storage/storageAccounts@2023-05-01' = {
  name: 'ctsync${uniqueString(resourceGroup().id)}'
  location: location // même région que le groupe de ressources
  kind: 'StorageV2'
  sku: { name: 'Standard_ZRS' }
}
```

Dockerfile:

```dockerfile
# Petite image de base pour l’agent
FROM node:24-slim
# Installer uniquement les dépendances de production
RUN npm ci --omit=dev
```

Terraform:

```hcl
# Groupe de ressources pour toutes les ressources de synchronisation
resource "azurerm_resource_group" "sync" {
  name     = "rg-sync"
  location = "westeurope" # conserver les données dans l’UE
}
```
