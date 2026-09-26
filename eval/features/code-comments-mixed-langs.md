# Infrastructure and service code

Go:

```go
package main

// Config holds the settings that are read at startup.
type Config struct {
	Endpoint string // URL of the sync service
	Workers  int    // number of parallel uploads
}
```

Rust:

```rust
/// Computes the hash of a block.
fn hash_block(data: &[u8]) -> [u8; 32] {
    // SHA-256 is fast enough for 4 MB blocks
    sha256(data)
}
```

Java:

```java
/**
 * Listens for changes and queues them for upload.
 */
public class ChangeListener {
    // Changes are batched every 500 ms
    private static final int BATCH_MS = 500;
}
```

SQL:

```sql
-- Find devices that have not synced for a week
SELECT device_id, last_sync
FROM devices
WHERE last_sync < DATEADD(day, -7, GETUTCDATE()); /* UTC on purpose */
```

YAML:

```yaml
# Settings for the sync agent
agent:
  workers: 8 # one per CPU core is a good start
  log_level: info
```

Bicep:

```bicep
// Storage account for the sync data
resource storage 'Microsoft.Storage/storageAccounts@2023-05-01' = {
  name: 'ctsync${uniqueString(resourceGroup().id)}'
  location: location // same region as the resource group
  kind: 'StorageV2'
  sku: { name: 'Standard_ZRS' }
}
```

Dockerfile:

```dockerfile
# Small base image for the agent
FROM node:24-slim
# Install only production dependencies
RUN npm ci --omit=dev
```

Terraform:

```hcl
# Resource group for all sync resources
resource "azurerm_resource_group" "sync" {
  name     = "rg-sync"
  location = "westeurope" # keep data in the EU
}
```
