# Settings reference

This page lists the options on the **Settings** page of the portal and in the configuration file.

## General

| Option | Values | Default | Description |
|--------|--------|---------|-------------|
| Mode | Run, Test, Off | Run | Controls whether jobs run, only simulate, or are paused. |
| Retry | Yes, No | Yes | Retries a failed job once after a short delay. |
| Schedule | Daily, Weekly | Daily | How often the cleanup job runs. |
| Owner | Any user | Current user | Receives alerts when a job fails. |
| Scope | Project, Account | Project | Where the setting applies. |

To change an option, select **Edit**, choose a value, and then select **Save**. Select **Cancel** to discard your changes.

## Notifications

Choose which events trigger a notification:

- Job failed
- Job succeeded after a retry
- Quota nearly reached

| Channel | Supported | Notes |
|---------|-----------|-------|
| Email | Yes | Sent to the owner. |
| Teams | Yes | Requires a webhook. |
| SMS | No | Planned. |

## Actions

| Button | Result |
|--------|--------|
| **Run now** | Starts the job immediately. |
| **Pause** | Stops new runs; running jobs finish. |
| **Reset** | Restores the default values. |
| **Export** | Downloads the settings as a file. |

> [!WARNING]
> **Reset** can't be undone.
