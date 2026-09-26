# <a id="using-the-net-sdk"></a>Utilisation du SDK .NET

```csharp
using Contoso.Sync;

/// <summary>
/// Charge l’exportation nocturne et indique le résultat.
/// </summary>
/// <param name="path">Chemin complet du fichier d’exportation.</param>
/// <returns><c>true</c> lorsque le chargement a réussi ; sinon <c>false</c>.</returns>
public static async Task<bool> UploadExportAsync(string path)
{
    // Un seul client par application suffit
    var client = new SyncClient(new Uri("https://sync.contoso.example"));

    /* Les exportations volumineuses sont automatiquement
       fractionnées en blocs. La taille des blocs peut être
       modifiée avec SyncClientOptions. */
    var result = await client.UploadAsync(path);

    return result.Succeeded; // false lorsque le quota est dépassé
}
```

Configuration dans `appsettings.json` :

```json
{
  "Sync": { "Endpoint": "https://sync.contoso.example", "Retries": 3 }
}
```
