# <a id="how-the-scheduler-picks-a-server"></a>Comment l’ordonnanceur choisit un serveur

L’ordonnanceur estime la charge de chaque serveur $s$ comme la somme pondérée de son utilisation du processeur $c_s$ et de sa longueur de file d’attente $q_s$ :

$$
L_s = \alpha \cdot c_s + (1 - \alpha) \cdot \frac{q_s}{q_{\max}}
$$

Le poids $\alpha$ est compris entre $0$ et $1$. Avec $\alpha = 0.7$, l’utilisation du processeur domine.

Une nouvelle tâche est envoyée au serveur ayant la plus petite charge, $\arg\min_s L_s$. Si deux serveurs ont la même charge, celui dont l’indice $i$ est le plus bas l’emporte.

Le temps d’attente attendu augmente avec $\frac{1}{1 - \rho}$, où $\rho$ est l’utilisation. Pour $\rho \to 1$, le temps d’attente est non borné.

Les formules mathématiques en mode affichage insérées dans le texte fonctionnent également : $$E = mc^2$$ est l’exemple classique.

La variance est $\sigma^2 = \frac{1}{n}\sum_{i=1}^{n}(x_i - \mu)^2$.
