# Windows line endings

This file uses CRLF line endings. They must survive translation byte for byte.

## Steps

1. Open **Control Panel**.
2. Select **Programs** > **Uninstall a program**.
3. Select *Contoso Sync* and then **Uninstall**.

```powershell
# Remove the leftover data folder
Remove-Item -Recurse "$env:LOCALAPPDATA\ContosoSync"
```

> [!NOTE]
> Your files in the cloud aren't deleted.
