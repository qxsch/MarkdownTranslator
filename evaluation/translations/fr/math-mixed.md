# <a id="capacity-planning"></a>Planification de la capacité

Une seule passerelle traite environ $r = 400$ requêtes par seconde. Pour une charge de pointe de $R$ requêtes par seconde, il vous faut $n = \lceil R / r \rceil$ passerelles.

Chaque passerelle coûte 120 $ par mois. Pour 2 000 requêtes par seconde, il vous faut 5 passerelles, ce qui coûte 600 $ par mois.

Bloc mathématique pour l’estimation du stockage :

$$
S = u \cdot f \cdot (1 + v)
$$

où $u$ correspond au nombre d’utilisateurs, $f$ au volume moyen de données par utilisateur et $v$ à la part des anciennes versions.

Avec 500 utilisateurs, 20 Go chacun et $v = 0.3$, il vous faut 13 To. À 20 $ par To, le stockage coûte 260 $ par mois.
