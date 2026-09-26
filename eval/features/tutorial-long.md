# Tutorial: Automate backups with Contoso Sync

In this tutorial, you set up a nightly backup of a project folder, test a restore and get notified when something fails.

## Part 1: Prepare the folder

Create a folder named `projects` in your home directory. Move the files that you want to back up into it. Avoid folders that contain build output, such as `node_modules` or `bin`, because they change often and use a lot of space.

Next, add a file named `.ctsyncignore` to exclude patterns:

```text
node_modules/
bin/
*.tmp
```

## Part 2: Create the backup job

1. Open **Settings** > **Backups**.
2. Select **New job**.
3. Enter a name, for example *Nightly projects*.
4. Under **Schedule**, select **Daily** and set the time to 02:00.
5. Under **Keep**, select how many versions to keep. Seven is a good start.
6. Select **Create**.

The job appears in the list with the status **Scheduled**.

## Part 3: Test a restore

A backup is only useful if you can restore it. Delete a test file, then:

1. Select the job and then **Restore**.
2. Pick yesterday's version.
3. Choose **Restore to original location**.

The file reappears within a few seconds.

## Part 4: Get notified

Under **Notifications**, turn on **E-mail me when a job fails**. You can also send events to a webhook:

```json
{
  "webhook": "https://hooks.example.com/backup",
  "events": ["job.failed", "job.succeeded"]
}
```

## Clean up

If you don't want to keep the job, select it and then **Delete**. Deleting the job doesn't delete existing backups; they expire according to the retention setting.

## Summary

You created a scheduled backup, tested a restore and set up notifications. Continue with [Share a folder with your team](share.md).
