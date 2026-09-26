# <a id="using-the-net-sdk"></a>Utilisation du SDK .NET

```csharp
using Contoso.Sync;

/// <summary>
/// Charge l’export nocturne et indique le résultat.
/// </summary>
/// <param name="path">Chemin d’accès complet du fichier d’export.</param>
/// <returns><c>true</c> si le chargement a réussi ; sinon <c>false</c>.</returns>
public static async Task<bool> UploadExportAsync(string path)
{
    // Un client par application suffit
    var client = new SyncClient(new Uri("https://sync.contoso.example"));

    /* Les exports volumineux sont automatiquement divisés
       en blocs. La taille de bloc peut être modifiée avec
       SyncClientOptions. */
    var result = await client.UploadAsync(path);

    return result.Succeeded; // false lorsque le quota est dépassé
}
```

Configuration dans `appsettings.json` :

```json
{
  "Sync": { "Endpoint": "https://sync.contoso.example", "Retries": 3 }
}
```
