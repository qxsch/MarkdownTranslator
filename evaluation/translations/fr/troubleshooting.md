# <a id="troubleshooting"></a>Résolution des problèmes

Utilisez cette page lorsque la synchronisation ne fonctionne pas comme prévu.

## <a id="error-0x8004de40-cant-connect-to-the-server"></a>Erreur 0x8004de40 : « Impossible de se connecter au serveur »

**Cause :** Un proxy ou un pare-feu bloque la connexion.

**Solution :**

1. Vérifiez que `sync.contoso.example` est accessible sur le port 443.
2. Si vous utilisez un proxy, définissez la variable d’environnement `HTTPS_PROXY`.
3. Redémarrez le client avec `ctsync restart`.

## <a id="files-stay-in-the-pending-state"></a>Les fichiers restent à l’état « En attente »

Cela se produit généralement lorsqu’un fichier est ouvert dans un autre programme. Fermez le programme, puis attendez quelques minutes. Si le fichier est toujours en attente, consultez le journal :

```powershell
Get-Content "$env:LOCALAPPDATA\ContosoSync\logs\ctsync.log" -Tail 50
```

## <a id="sync-is-slow"></a>La synchronisation est lente

- Les fichiers volumineux sont chargés par blocs de 4 Mo. Un fichier de 10 Go prend du temps.
- Les logiciels antivirus peuvent analyser chaque fichier deux fois. Excluez le dossier de synchronisation de l’analyse en temps réel.
- Vérifiez la limite de bande passante sous **Paramètres** > **Réseau**.

## <a id="the-app-crashes-at-startup"></a>L’application se ferme de façon inattendue au démarrage

Supprimez le fichier de paramètres `settings.json` dans `%APPDATA%\ContosoSync` et relancez l’application. Vos fichiers ne sont pas affectés.

## <a id="still-stuck"></a>Le problème persiste ?

Collectez les diagnostics avec `ctsync diag --zip` et joignez le fichier `diag.zip` à une demande de support.
