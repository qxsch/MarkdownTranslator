# Environment and identifiers

Set `CTSYNC_HOME` to change the data folder. On Windows, the default is `%LOCALAPPDATA%\ContosoSync`; on Linux it's `$HOME/.ctsync`.

The client reads the proxy from HTTPS_PROXY and ignores NO_PROXY entries that contain wildcards.

Use the cmdlet Get-CtSyncStatus to check the state from PowerShell, or call the method getSyncStatus() from the JavaScript SDK. The Python SDK names it get_sync_status.

The flag --max-retries accepts values from 0 to 10. The option maxRetries in `config.toml` does the same.

Connection strings look like `Endpoint=https://sync.contoso.example;SharedAccessKey=<key>`. Replace `${TENANT_ID}` with your tenant ID, for example 00000000-0000-0000-0000-000000000000.

Version 3.2.0-beta.1 introduced the setting `upload.parallelism`; version v3.2.1 raised its default to 8.
