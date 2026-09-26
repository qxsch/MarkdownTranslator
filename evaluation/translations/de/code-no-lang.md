# <a id="output-samples"></a>Ausgabebeispiele

Ein Code-Fence ohne Sprache:

```
ctsync status
Folder        State     Files
Documents     Synced    1,204
Pictures      Pending   37
```

Ein Tilde-Fence:

~~~
[2026-08-14 10:02:11] INFO  Upload started: report.pdf
[2026-08-14 10:02:14] WARN  Slow connection detected
~~~

Ein Code-Fence mit einer unbekannten Sprache:

```ctsyncrc
# This comment is in an unknown format and must stay unchanged
workers = 8
```

Ein eingerückter Block:

    # Indented code is never translated
    ctsync reset --force

Ein Code-Fence innerhalb einer Liste:

1. Führen Sie den Befehl aus:

   ```bash
   # Show the version
   ctsync --version
   ```

2. Überprüfen Sie, ob die Ausgabe mit `3.` beginnt.

Ein Code-Fence innerhalb eines Zitats:

> ```text
> Keep this text exactly as it is.
> ```
