# <a id="queueing-model-for-uploads"></a>Warteschlangenmodell für Uploads

Jeder Upload-Worker $w$ bedient Anforderungen mit der Rate $\mu$, und Anforderungen treffen mit der Rate $\lambda$ ein. Die Auslastung beträgt $\rho = \lambda / \mu$ und muss unter $1$ bleiben, damit die Warteschlange stabil ist.

## <a id="key-formulas"></a>Wichtige Formeln

- Mittlere Anzahl von Anforderungen im System: $L = \frac{\rho}{1 - \rho}$
- Mittlere Zeit im System: $W = \frac{1}{\mu - \lambda}$
- Wahrscheinlichkeit, dass eine Anforderung warten muss: $P_{wait} = \rho^k$ bei $k$ Workern im vereinfachten Modell

| Symbol | Bedeutung | Typischer Wert |
|---|---|---|
| $\lambda$ | Ankunftsrate | 120 Anforderungen pro Sekunde |
| $\mu$ | Bedienrate pro Worker | 40 Anforderungen pro Sekunde |
| $k$ | Anzahl der Worker | 4 |
| $\rho$ | Auslastung | $0.75$ |

Mit $k = 4$ Workern und den obigen Werten gilt $\rho = 120 / (4 \cdot 40) = 0.75$; daher ist die Warteschlange stabil.

## <a id="tuning"></a>Optimierung

Wenn das Latenzziel $W \le 50$ ms beträgt, fügen Sie Worker hinzu, bis $\mu_{total} - \lambda \ge 20$ Anforderungen pro Sekunde gilt. Eine Verdoppelung von $k$ halbiert $\rho$, erhöht die Kosten jedoch linear.

Auch die Varianz der Bedienzeit, $\sigma_s^2$, ist relevant: Gemäß der Pollaczek-Khinchine-Formel wächst die Wartezeit mit $1 + c_s^2$, wobei $c_s$ der Variationskoeffizient ist.
