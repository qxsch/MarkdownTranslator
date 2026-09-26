# Queueing model for uploads

Each upload worker $w$ serves requests at rate $\mu$, and requests arrive at rate $\lambda$. The utilization is $\rho = \lambda / \mu$, which must stay below $1$ for the queue to be stable.

## Key formulas

- Mean number of requests in the system: $L = \frac{\rho}{1 - \rho}$
- Mean time in the system: $W = \frac{1}{\mu - \lambda}$
- Probability that a request waits: $P_{wait} = \rho^k$ for $k$ workers in the simplified model

| Symbol | Meaning | Typical value |
|---|---|---|
| $\lambda$ | Arrival rate | 120 requests per second |
| $\mu$ | Service rate per worker | 40 requests per second |
| $k$ | Number of workers | 4 |
| $\rho$ | Utilization | $0.75$ |

With $k = 4$ workers and the values above, $\rho = 120 / (4 \cdot 40) = 0.75$, so the queue is stable.

## Tuning

If the latency target is $W \le 50$ ms, add workers until $\mu_{total} - \lambda \ge 20$ requests per second. Doubling $k$ halves $\rho$ but increases the cost linearly.

The variance of the service time, $\sigma_s^2$, matters too: by the Pollaczek-Khinchine formula the waiting time grows with $1 + c_s^2$, where $c_s$ is the coefficient of variation.
