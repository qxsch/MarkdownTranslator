# Configure the upload queue

The `UploadQueue` class exposes `maxParallel`, `retryDelayMs` and `onProgress`. Set `maxParallel` to `0` to use the number of CPU cores.

Call `queue.pause()` before you call `queue.flush()`, otherwise `flush()` throws `QueueBusyError`.

The events `upload:start`, `upload:done` and `upload:error` are emitted in that order. Listen to `upload:error` with `queue.on('upload:error', handler)`.

If `retryDelayMs` is `null`, the default of `500` is used. Values above `60000` are capped.

The `--queue-size` flag and the `CTSYNC_QUEUE_SIZE` variable override `queueSize` in `config.toml`.
