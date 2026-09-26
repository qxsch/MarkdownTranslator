# Getting started with Contoso Sync

Contoso Sync keeps a local folder and a cloud share in sync. This guide takes about ten minutes.

## Prerequisites

Before you begin, make sure that you have:

- A Contoso account with the **Contributor** role
- Windows 11 or macOS 14 or later
- At least 2 GB of free disk space

## Install the client

1. Download `ContosoSync-Setup.exe` from the [download page](https://contoso.example/download).
2. Run the installer and select **Next**.
3. When you're asked for a folder, keep the default `C:\Users\<you>\ContosoSync` or select **Browse**.
4. Select **Install**, then **Finish**.

> [!TIP]
> You can install the client silently with `ContosoSync-Setup.exe /quiet`.

## Sign in

Open the app from the Start menu. Enter your work e-mail address and select **Sign in**. If your organization uses multifactor authentication, approve the request on your phone.

## Choose what to sync

| Option | Default | Description |
|---|---|---|
| Files on demand | On | Files are downloaded only when you open them |
| Sync on metered networks | Off | Pauses sync on mobile hotspots |
| Keep | 30 days | How long deleted files stay in the recycle bin |

## Next steps

- [Share a folder](#choose-what-to-sync) with your team
- Read the [troubleshooting guide](troubleshooting.md)
