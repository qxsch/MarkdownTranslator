# <a id="performance-results"></a>Résultats de performance

Le débit de synchronisation s’est amélioré de 40 %[^1] par rapport à la version précédente. Le test a utilisé le benchmark standard[^bench] sur trois machines.

La latence est restée inférieure à 200 ms pour 99 % des requêtes[^p99].

## <a id="method"></a>Méthode

Chaque exécution a commencé avec un cache vide.[^cache] Nous avons répété chaque mesure cinq fois et indiquons la médiane.

[^1]: Mesuré sur une connexion à 1 Gbit/s avec 10 000 fichiers de 1 Mo chacun.
[^bench]: Le benchmark est décrit dans `bench/README.md`. Il crée des fichiers avec un contenu aléatoire.
[^p99]: Le 99e percentile, mesuré au niveau de la passerelle.
[^cache]: Pour vider le cache, exécutez `ctsync reset --cache-only`. Un deuxième paragraphe de la note de
    bas de page explique pourquoi un cache froid est important : la première analyse lit chaque fichier.
