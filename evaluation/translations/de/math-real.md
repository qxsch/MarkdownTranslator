# <a id="how-the-scheduler-picks-a-server"></a>Wie der Scheduler einen Server auswählt

Der Scheduler schätzt die Last jedes Servers $s$ als gewichtete Summe seiner CPU-Auslastung $c_s$ und seiner Warteschlangenlänge $q_s$:

$$
L_s = \alpha \cdot c_s + (1 - \alpha) \cdot \frac{q_s}{q_{\max}}
$$

Die Gewichtung $\alpha$ liegt zwischen $0$ und $1$. Bei $\alpha = 0.7$ dominiert die CPU-Auslastung.

Ein neuer Auftrag wird an den Server mit der kleinsten Last, $\arg\min_s L_s$, gesendet. Wenn zwei Server die gleiche Last aufweisen, gewinnt der Server mit dem niedrigeren Index $i$.

Die erwartete Wartezeit wächst mit $\frac{1}{1 - \rho}$, wobei $\rho$ die Auslastung ist. Für $\rho \to 1$ ist die Wartezeit unbeschränkt.

Auch inline eingebundene abgesetzte Mathematik funktioniert: $$E = mc^2$$ ist das klassische Beispiel.

Die Varianz beträgt $\sigma^2 = \frac{1}{n}\sum_{i=1}^{n}(x_i - \mu)^2$.
