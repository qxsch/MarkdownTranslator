# <a id="how-the-scheduler-picks-a-server"></a>Wie der Scheduler einen Server auswählt

Der Scheduler schätzt die Last jedes Servers $s$ als gewichtete Summe seiner Prozessorauslastung $c_s$ und seiner Warteschlangenlänge $q_s$:

$$
L_s = \alpha \cdot c_s + (1 - \alpha) \cdot \frac{q_s}{q_{\max}}
$$

Der Gewichtungsfaktor $\alpha$ liegt zwischen $0$ und $1$. Bei $\alpha = 0.7$ dominiert die Prozessorauslastung.

Ein neuer Job wird dem Server mit der geringsten Last, $\arg\min_s L_s$, zugewiesen. Wenn zwei Server dieselbe Last haben, gewinnt der mit dem niedrigeren Index $i$.

Die erwartete Wartezeit wächst mit $\frac{1}{1 - \rho}$, wobei $\rho$ die Auslastung ist. Für $\rho \to 1$ ist die Wartezeit unbeschränkt.

Display-Mathematik im Fließtext funktioniert ebenfalls: $$E = mc^2$$ ist das klassische Beispiel.

Die Varianz ist $\sigma^2 = \frac{1}{n}\sum_{i=1}^{n}(x_i - \mu)^2$.
