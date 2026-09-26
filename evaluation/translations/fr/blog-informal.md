# <a id="why-we-rewrote-our-sync-engine-and-what-went-wrong"></a>Pourquoi nous avons réécrit notre moteur de synchronisation (et ce qui s’est mal passé)

Soyons honnêtes : l’ancien moteur était une vraie galère. Il fonctionnait, plus ou moins, mais chaque nouvelle fonctionnalité était un calvaire à ajouter. Alors, au printemps dernier, nous avons pris notre courage à deux mains et sommes repartis de zéro.

## <a id="the-first-attempt-was-a-flop"></a>La première tentative a fait un flop

Nous pensions boucler ça en trois mois. Spoiler : ça n’a pas été le cas. Notre premier prototype était ultrarapide sur nos ordinateurs portables, puis s’est effondré dès que nous lui avons donné de vraies données client à traiter. Des dossiers avec 200 000 fichiers ? C’était fini.

## <a id="what-finally-clicked"></a>Le déclic

Le déclic est venu quand nous avons arrêté de comparer les fichiers un par un et commencé à comparer les hachages de dossiers entiers. Si le hachage correspond, nous ignorons le dossier. Simple, non ? Il nous a fallu beaucoup trop de temps pour nous en rendre compte.

## <a id="numbers-because-you-asked"></a>Les chiffres, puisque tu les as demandés

- Analyse initiale de 1 million de fichiers : 41 minutes avant, 6 minutes maintenant
- Utilisation de la mémoire : passée de 1,2 Go à 300 Mo
- Rapports de plantage : en baisse de 80 %

## <a id="whats-next"></a>La suite

Nous n’avons pas encore terminé. La prochaine étape, c’est la synchronisation pair à pair sur le réseau local, pour que tes fichiers ne fassent pas l’aller-retour par le cloud quand ton collègue est assis juste à côté de toi. Reste à l’écoute, et si tu tombes sur un bug, fais-nous signe !
