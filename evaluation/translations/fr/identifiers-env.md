# <a id="environment-and-identifiers"></a>Environnement et identifiants

Définissez `CTSYNC_HOME` pour modifier le dossier de données. Sous Windows, la valeur par défaut est `%LOCALAPPDATA%\ContosoSync` ; sous Linux, il s’agit de `$HOME/.ctsync`.

Le client lit le proxy à partir de HTTPS_PROXY et ignore les entrées NO_PROXY qui contiennent des caractères génériques.

Utilisez l’applet de commande Get-CtSyncStatus pour vérifier l’état depuis PowerShell, ou appelez la méthode getSyncStatus() du SDK JavaScript. Le SDK Python l’appelle get_sync_status.

L’indicateur --max-retries accepte des valeurs de 0 à 10. L’option maxRetries dans `config.toml` fait de même.

Les chaînes de connexion se présentent sous la forme `Endpoint=https://sync.contoso.example;SharedAccessKey=<key>`. Remplacez `${TENANT_ID}` par votre ID de locataire, par exemple 00000000-0000-0000-0000-000000000000.

La version 3.2.0-beta.1 a introduit le paramètre `upload.parallelism` ; la version v3.2.1 a porté sa valeur par défaut à 8.
