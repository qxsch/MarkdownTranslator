# <a id="why-we-rewrote-our-sync-engine-and-what-went-wrong"></a>Pourquoi on a réécrit notre moteur de synchronisation (et ce qui a mal tourné)

Soyons honnêtes : l’ancien moteur était une vraie galère. Il fonctionnait, plus ou moins, mais chaque nouvelle fonctionnalité relevait du parcours du combattant. Alors, au printemps dernier, on a pris notre courage à deux mains et on est repartis de zéro.

## <a id="the-first-attempt-was-a-flop"></a>La première tentative a fait un flop

On pensait boucler ça en trois mois. Spoiler : ça n’a pas été le cas. Notre premier prototype était ultra-rapide sur nos ordinateurs portables et s’est effondré dès qu’on lui a soumis de vraies données client. Des dossiers avec 200 000 fichiers ? Partie terminée.

## <a id="what-finally-clicked"></a>Le déclic

La percée est arrivée quand on a arrêté de comparer les fichiers un par un pour commencer à comparer les hachages de dossiers entiers. Si le hachage correspond, on ignore le dossier. Simple, non ? Il nous a fallu beaucoup trop longtemps pour le voir.

## <a id="numbers-because-you-asked"></a>Des chiffres, puisque tu l’as demandé

- Analyse initiale de 1 million de fichiers : 41 minutes avant, 6 minutes maintenant
- Utilisation de la mémoire : passée de 1,2 Go à 300 Mo
- Rapports de plantage : en baisse de 80 %

## <a id="whats-next"></a>La suite

On n’a pas encore terminé. La prochaine étape, c’est la synchronisation pair à pair sur le réseau local, pour que tes fichiers ne fassent pas un aller-retour par le cloud quand ton collègue est assis juste à côté de toi. Reste à l’écoute, et si tu tombes sur un bug, fais-nous signe !
