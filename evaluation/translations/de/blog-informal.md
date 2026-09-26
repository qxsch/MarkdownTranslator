# <a id="why-we-rewrote-our-sync-engine-and-what-went-wrong"></a>Warum wir unsere Sync-Engine neu geschrieben haben (und was schiefgelaufen ist)

Seien wir ehrlich: Die alte Engine war eine Qual. Sie funktionierte irgendwie, aber jedes neue Feature fühlte sich an, als müsste man Zähne ziehen. Also haben wir letzten Frühling in den sauren Apfel gebissen und von vorn angefangen.

## <a id="the-first-attempt-was-a-flop"></a>Der erste Versuch war ein Reinfall

Wir dachten, wir würden das in drei Monaten schaffen. Spoiler: Haben wir nicht. Unser erster Prototyp war auf unseren Laptops rasend schnell und brach in dem Moment zusammen, als wir echte Kundendaten darauf losließen. Ordner mit 200.000 Dateien? Keine Chance.

## <a id="what-finally-clicked"></a>Wobei es schließlich Klick gemacht hat

Der Durchbruch kam, als wir aufgehört haben, Dateien einzeln zu vergleichen, und stattdessen angefangen haben, Hashes ganzer Ordner zu vergleichen. Wenn der Hash übereinstimmt, überspringen wir den Ordner. Einfach, oder? Wir haben viel zu lange gebraucht, um darauf zu kommen.

## <a id="numbers-because-you-asked"></a>Zahlen, weil du gefragt hast

- Erst-Scan von 1 Million Dateien: vorher 41 Minuten, jetzt 6 Minuten
- Speichernutzung: von 1,2 GB auf 300 MB gesunken
- Absturzberichte: um 80 % gesunken

## <a id="whats-next"></a>Was als Nächstes kommt

Wir sind noch nicht fertig. Als Nächstes kommt die Peer-to-Peer-Synchronisierung im lokalen Netzwerk, damit deine Dateien keinen Umweg durch die Cloud nehmen müssen, wenn dein Kollege direkt neben dir sitzt. Bleib dran, und wenn du auf einen Bug stößt, sag uns Bescheid!
