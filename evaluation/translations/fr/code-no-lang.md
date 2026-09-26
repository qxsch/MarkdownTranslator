# <a id="output-samples"></a>Exemples de sortie

Une clôture sans langage :

```
ctsync status
Folder        State     Files
Documents     Synced    1,204
Pictures      Pending   37
```

Une clôture par tildes :

~~~
[2026-08-14 10:02:11] INFO  Upload started: report.pdf
[2026-08-14 10:02:14] WARN  Slow connection detected
~~~

Une clôture avec un langage inconnu :

```ctsyncrc
# This comment is in an unknown format and must stay unchanged
workers = 8
```

Un bloc indenté :

    # Indented code is never translated
    ctsync reset --force

Une clôture dans une liste :

1. Exécutez la commande :

   ```bash
   # Afficher la version
   ctsync --version
   ```

2. Vérifiez que la sortie commence par `3.`.

Une clôture dans une citation :

> ```text
> Keep this text exactly as it is.
> ```
