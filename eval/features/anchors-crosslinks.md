# Administrator guide

This guide links to its own sections a lot. Start with [Plan the rollout](#plan-the-rollout), then read [Assign licenses](#assign-licenses).

## Plan the rollout

Decide which groups get the client first. See [Pilot groups](#pilot-groups) for recommendations and [Roll back](#roll-back) if something goes wrong.

### Pilot groups

Start with a group of 20 to 50 users. Their feedback helps you tune the [default settings](#default-settings).

## Assign licenses

Licenses are assigned per user. Group-based assignment is described in [Automate assignment](#automate-assignment).

### Automate assignment

Use the admin API to assign licenses from your HR system. The API is described in the [error reference](api-errors-table.md#handling-conflicts).

## Default settings

Push default settings with your device management tool. Settings that users can't change are marked as **locked**.

## Roll back

To roll back, uninstall the client and remove the policy. Files stay in the cloud. Go back to the [top](#administrator-guide).
