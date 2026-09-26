# Output samples

A fence without a language:

```
ctsync status
Folder        State     Files
Documents     Synced    1,204
Pictures      Pending   37
```

A tilde fence:

~~~
[2026-08-14 10:02:11] INFO  Upload started: report.pdf
[2026-08-14 10:02:14] WARN  Slow connection detected
~~~

A fence with an unknown language:

```ctsyncrc
# This comment is in an unknown format and must stay unchanged
workers = 8
```

An indented block:

    # Indented code is never translated
    ctsync reset --force

A fence inside a list:

1. Run the command:

   ```bash
   # Show the version
   ctsync --version
   ```

2. Check that the output starts with `3.`.

A fence inside a quote:

> ```text
> Keep this text exactly as it is.
> ```
