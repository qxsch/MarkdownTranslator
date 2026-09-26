# Files and folders

The client stores its settings in `settings.json` and its logs in the `logs` folder. On Windows, the full path is `C:\Users\<name>\AppData\Local\ContosoSync\logs\ctsync.log`; on Linux it's `~/.local/share/ctsync/ctsync.log`.

To reset the client, delete settings.json and restart. Don't delete sync.db, because it contains the list of synced files.

Open the file README.md in the root of the repository and follow the steps. The script ./scripts/install.sh installs the dependencies, and deploy.ps1 publishes the build to the folder dist/.

Images such as logo.png, banner@2x.jpg and icons/app.ico are copied as is.

| File | Purpose |
|---|---|
| `config.toml` | Main configuration |
| `ignore.txt` | Patterns to exclude |
| `Dockerfile` | Container build |
| `.env.example` | Template for environment variables |

Log files older than seven days are compressed to `ctsync-2026-08-01.log.gz`.
