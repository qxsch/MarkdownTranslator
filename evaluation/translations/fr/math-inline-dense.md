# <a id="queueing-model-for-uploads"></a>Modèle de file d’attente pour les chargements

Chaque agent de chargement $w$ traite les requêtes au taux $\mu$, et les requêtes arrivent au taux $\lambda$. L’utilisation est $\rho = \lambda / \mu$, qui doit rester inférieure à $1$ pour que la file d’attente soit stable.

## <a id="key-formulas"></a>Formules clés

- Nombre moyen de requêtes dans le système : $L = \frac{\rho}{1 - \rho}$
- Temps moyen dans le système : $W = \frac{1}{\mu - \lambda}$
- Probabilité qu’une requête attende : $P_{wait} = \rho^k$ pour $k$ agents dans le modèle simplifié

| Symbole | Signification | Valeur type |
|---|---|---|
| $\lambda$ | Taux d’arrivée | 120 requêtes par seconde |
| $\mu$ | Taux de service par agent | 40 requêtes par seconde |
| $k$ | Nombre d’agents | 4 |
| $\rho$ | Utilisation | $0.75$ |

Avec $k = 4$ agents et les valeurs ci-dessus, $\rho = 120 / (4 \cdot 40) = 0.75$ ; la file d’attente est donc stable.

## <a id="tuning"></a>Réglage

Si l’objectif de latence est de $W \le 50$ ms, ajoutez des agents jusqu’à ce que la condition $\mu_{total} - \lambda \ge 20$ requêtes par seconde soit satisfaite. Doubler $k$ divise $\rho$ par deux, mais augmente le coût linéairement.

La variance du temps de service, $\sigma_s^2$, a également son importance : selon la formule de Pollaczek-Khinchine, le temps d’attente augmente avec $1 + c_s^2$, où $c_s$ est le coefficient de variation.
