# <a id="configure-the-upload-queue"></a>Upload-Warteschlange konfigurieren

Die Klasse `UploadQueue` stellt `maxParallel`, `retryDelayMs` und `onProgress` bereit. Legen Sie `maxParallel` auf `0` fest, um die Anzahl der CPU-Kerne zu verwenden.

Rufen Sie `queue.pause()` auf, bevor Sie `queue.flush()` aufrufen, andernfalls löst `flush()` `QueueBusyError` aus.

Die Ereignisse `upload:start`, `upload:done` und `upload:error` werden in dieser Reihenfolge ausgelöst. Registrieren Sie mit `queue.on('upload:error', handler)` einen Listener für `upload:error`.

Wenn `retryDelayMs` `null` ist, wird der Standardwert `500` verwendet. Werte über `60000` werden begrenzt.

Das Flag `--queue-size` und die Variable `CTSYNC_QUEUE_SIZE` überschreiben `queueSize` in `config.toml`.
