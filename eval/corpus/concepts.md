# Consistency levels in distributed databases

Distributed databases replicate data across several regions so that applications stay available even when an entire datacenter goes down. Replication, however, forces a trade-off: the stronger the guarantees about the order and visibility of writes, the higher the latency and the lower the availability during network partitions.

## Why consistency is not binary

It is tempting to think of consistency as a switch that is either on or off. In practice, most systems offer a spectrum of levels, each of which defines precisely what a reader may observe after a writer has committed a change.

With *strong* consistency, every read returns the most recent committed write, regardless of the region it is served from. This is the easiest model to reason about, but it requires every write to be acknowledged by a quorum of replicas before it is considered committed, which adds round trips between regions.

With *eventual* consistency, replicas converge over time, but a reader might temporarily see older values, or even see writes out of order. Applications that display, for example, the number of likes on a post can usually tolerate this, because a slightly outdated count does no harm.

Between these extremes lie models such as *bounded staleness*, where reads may lag behind writes by at most a configured number of versions or a time interval, and *session* consistency, which guarantees that a client always reads its own writes, while other clients may still see older data.

## Choosing a level

There is no universally correct choice. Before you decide, consider the following questions:

1. What is the business impact if a user sees data that is a few seconds old?
2. Do users mostly read the data they have written themselves, or data written by others?
3. How much additional latency is acceptable for write operations that span multiple regions?

If the answers are unclear, start with session consistency. It provides intuitive behavior for the most common interaction patterns, while keeping latency and cost close to those of eventual consistency.

> Keep in mind that the consistency level configured on the account is only a default. Individual requests can often relax it, but they can never make it stronger than the account setting.

## Summary

Consistency levels let you balance correctness, latency and availability on a per-application basis. Understanding the guarantees of each level, and the failure modes they protect against, is essential for designing systems that behave predictably under load and during outages.
