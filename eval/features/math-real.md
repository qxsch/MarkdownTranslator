# How the scheduler picks a server

The scheduler estimates the load of each server $s$ as a weighted sum of its CPU usage $c_s$ and its queue length $q_s$:

$$
L_s = \alpha \cdot c_s + (1 - \alpha) \cdot \frac{q_s}{q_{\max}}
$$

The weight $\alpha$ is between $0$ and $1$. With $\alpha = 0.7$, CPU usage dominates.

A new job goes to the server with the smallest load, $\arg\min_s L_s$. If two servers have the same load, the one with the lower index $i$ wins.

The expected waiting time grows with $\frac{1}{1 - \rho}$, where $\rho$ is the utilization. For $\rho \to 1$ the waiting time is unbounded.

Inline display math also works: $$E = mc^2$$ is the classic example.

The variance is $\sigma^2 = \frac{1}{n}\sum_{i=1}^{n}(x_i - \mu)^2$.
