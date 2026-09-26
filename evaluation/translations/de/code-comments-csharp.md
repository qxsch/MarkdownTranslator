# <a id="using-the-net-sdk"></a>Verwenden des .NET-SDKs

```csharp
using Contoso.Sync;

/// <summary>
/// Lädt den nächtlichen Export hoch und meldet das Ergebnis.
/// </summary>
/// <param name="path">Vollständiger Pfad der Exportdatei.</param>
/// <returns><c>true</c>, wenn der Upload erfolgreich war; andernfalls <c>false</c>.</returns>
public static async Task<bool> UploadExportAsync(string path)
{
    // Ein Client pro Anwendung genügt
    var client = new SyncClient(new Uri("https://sync.contoso.example"));

    /* Große Exporte werden automatisch in Blöcke
       aufgeteilt. Die Blockgröße kann mit SyncClientOptions
       geändert werden. */
    var result = await client.UploadAsync(path);

    return result.Succeeded; // false, wenn das Kontingent überschritten wird
}
```

Konfiguration in `appsettings.json`:

```json
{
  "Sync": { "Endpoint": "https://sync.contoso.example", "Retries": 3 }
}
```
