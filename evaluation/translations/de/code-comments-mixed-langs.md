# <a id="infrastructure-and-service-code"></a>Infrastruktur- und Service-Code

Go:

```go
package main

// Config enthält die Einstellungen, die beim Start gelesen werden.
type Config struct {
	Endpoint string // URL des Synchronisierungsdiensts
	Workers  int    // Anzahl paralleler Uploads
}
```

Rust:

```rust
/// Berechnet den Hash eines Blocks.
fn hash_block(data: &[u8]) -> [u8; 32] {
    // SHA-256 ist schnell genug für 4-MB-Blöcke
    sha256(data)
}
```

Java:

```java
/**
 * Überwacht Änderungen und stellt sie für den Upload in eine Warteschlange.
 */
public class ChangeListener {
    // Änderungen werden alle 500 ms gebündelt
    private static final int BATCH_MS = 500;
}
```

SQL:

```sql
-- Geräte suchen, die seit einer Woche nicht synchronisiert wurden
SELECT device_id, last_sync
FROM devices
WHERE last_sync < DATEADD(day, -7, GETUTCDATE()); /* bewusst UTC */
```

YAML:

```yaml
# Einstellungen für den Synchronisierungs-Agenten
agent:
  workers: 8 # Einer pro CPU-Kern ist ein guter Ausgangspunkt
  log_level: info
```

Bicep:

```bicep
// Speicherkonto für die Synchronisierungsdaten
resource storage 'Microsoft.Storage/storageAccounts@2023-05-01' = {
  name: 'ctsync${uniqueString(resourceGroup().id)}'
  location: location // dieselbe Region wie die Ressourcengruppe
  kind: 'StorageV2'
  sku: { name: 'Standard_ZRS' }
}
```

Dockerfile:

```dockerfile
# Kleines Basis-Image für den Agenten
FROM node:24-slim
# Nur Produktionsabhängigkeiten installieren
RUN npm ci --omit=dev
```

Terraform:

```hcl
# Ressourcengruppe für alle Synchronisierungsressourcen
resource "azurerm_resource_group" "sync" {
  name     = "rg-sync"
  location = "westeurope" # Daten in der EU belassen
}
```
