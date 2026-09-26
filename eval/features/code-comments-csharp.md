# Using the .NET SDK

```csharp
using Contoso.Sync;

/// <summary>
/// Uploads the nightly export and reports the result.
/// </summary>
/// <param name="path">Full path of the export file.</param>
/// <returns><c>true</c> when the upload succeeded; otherwise <c>false</c>.</returns>
public static async Task<bool> UploadExportAsync(string path)
{
    // One client per application is enough
    var client = new SyncClient(new Uri("https://sync.contoso.example"));

    /* Large exports are split into blocks automatically.
       The block size can be changed with SyncClientOptions. */
    var result = await client.UploadAsync(path);

    return result.Succeeded; // false when the quota is exceeded
}
```

Configuration in `appsettings.json`:

```json
{
  "Sync": { "Endpoint": "https://sync.contoso.example", "Retries": 3 }
}
```
