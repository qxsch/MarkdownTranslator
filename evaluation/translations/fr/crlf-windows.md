# <a id="windows-line-endings"></a>Fins de ligne Windows

Ce fichier utilise des fins de ligne CRLF. Elles doivent être préservées à l’octet près lors de la traduction.

## <a id="steps"></a>Étapes

1. Ouvrez le **Panneau de configuration**.
2. Sélectionnez **Programmes** > **Désinstaller un programme**.
3. Sélectionnez *Contoso Sync*, puis **Désinstaller**.

```powershell
# Supprimer le dossier de données résiduelles
Remove-Item -Recurse "$env:LOCALAPPDATA\ContosoSync"
```

> [!NOTE]
> Vos fichiers dans le cloud ne sont pas supprimés.
