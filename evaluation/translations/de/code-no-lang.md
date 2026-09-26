# <a id="output-samples"></a>Ausgabebeispiele

Eine Codeblock-Begrenzung ohne Sprache:

```
ctsync status
Folder        State     Files
Documents     Synced    1,204
Pictures      Pending   37
```

Eine Tilde-Begrenzung:

~~~
[2026-08-14 10:02:11] INFO  Upload started: report.pdf
[2026-08-14 10:02:14] WARN  Slow connection detected
~~~

Eine Codeblock-Begrenzung mit unbekannter Sprache:

```ctsyncrc
# This comment is in an unknown format and must stay unchanged
workers = 8
```

Ein eingerückter Block:

    # Indented code is never translated
    ctsync reset --force

Eine Codeblock-Begrenzung in einer Liste:

1. Führen Sie den Befehl aus:

   ```bash
   # Version anzeigen
   ctsync --version
   ```

2. Überprüfen Sie, ob die Ausgabe mit `3.` beginnt.

Eine Codeblock-Begrenzung in einem Zitat:

> ```text
> Keep this text exactly as it is.
> ```
