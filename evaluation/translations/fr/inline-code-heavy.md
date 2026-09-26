# <a id="configure-the-upload-queue"></a>Configurer la file d’attente de chargement

La classe `UploadQueue` expose `maxParallel`, `retryDelayMs` et `onProgress`. Définissez `maxParallel` sur `0` pour utiliser le nombre de cœurs de processeur.

Appelez `queue.pause()` avant d’appeler `queue.flush()` ; sinon, `flush()` lève `QueueBusyError`.

Les événements `upload:start`, `upload:done` et `upload:error` sont émis dans cet ordre. Abonnez-vous à `upload:error` avec `queue.on('upload:error', handler)`.

Si `retryDelayMs` est `null`, la valeur par défaut de `500` est utilisée. Les valeurs supérieures à `60000` sont plafonnées.

L’indicateur `--queue-size` et la variable `CTSYNC_QUEUE_SIZE` remplacent la valeur de `queueSize` définie dans `config.toml`.
