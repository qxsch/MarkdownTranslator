# Capacity planning

A single gateway handles about $r = 400$ requests per second. For a peak load of $R$ requests per second you need $n = \lceil R / r \rceil$ gateways.

Each gateway costs $120 per month. For 2,000 requests per second, you need 5 gateways, which costs $600 per month.

Block math for the storage estimate:

$$
S = u \cdot f \cdot (1 + v)
$$

where $u$ is the number of users, $f$ the average data per user and $v$ the share of old versions.

With 500 users, 20 GB each and $v = 0.3$, you need 13 TB. At $20 per TB, storage costs $260 per month.
