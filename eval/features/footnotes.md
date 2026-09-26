# Performance results

Sync throughput improved by 40%[^1] compared with the previous release. The test used the standard benchmark[^bench] on three machines.

Latency stayed below 200 ms for 99% of requests[^p99].

## Method

Each run started with an empty cache.[^cache] We repeated every measurement five times and report the median.

[^1]: Measured on a 1 Gbit/s connection with 10,000 files of 1 MB each.
[^bench]: The benchmark is described in `bench/README.md`. It creates files with random content.
[^p99]: The 99th percentile, measured at the gateway.
[^cache]: To clear the cache, run `ctsync reset --cache-only`.
    A second paragraph in the footnote explains why a cold cache matters: the first scan reads every file.
