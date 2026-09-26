# Troubleshooting

Use this page when sync doesn't work as expected.

## Error 0x8004de40: "Can't connect to the server"

**Cause:** A proxy or firewall blocks the connection.

**Solution:**

1. Check that `sync.contoso.example` is reachable on port 443.
2. If you use a proxy, set the `HTTPS_PROXY` environment variable.
3. Restart the client with `ctsync restart`.

## Files stay in the "Pending" state

This usually happens when a file is open in another program. Close the program, then wait a few minutes. If the file is still pending, check the log:

```powershell
Get-Content "$env:LOCALAPPDATA\ContosoSync\logs\ctsync.log" -Tail 50
```

## Sync is slow

- Large files are uploaded in blocks of 4 MB. A 10 GB file takes a while.
- Antivirus software can scan every file twice. Exclude the sync folder from real-time scanning.
- Check the bandwidth limit under **Settings** > **Network**.

## The app crashes at startup

Delete the settings file `settings.json` in `%APPDATA%\ContosoSync` and start the app again. Your files aren't affected.

## Still stuck?

Collect diagnostics with `ctsync diag --zip` and attach the file `diag.zip` to a support request.
