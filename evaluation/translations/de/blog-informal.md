# <a id="why-we-rewrote-our-sync-engine-and-what-went-wrong"></a>Warum wir unsere Synchronisierungs-Engine neu geschrieben haben (und was schiefgelaufen ist)

Seien wir ehrlich: Die alte Engine war eine echte Qual. Sie funktionierte, irgendwie, aber jedes neue Feature fühlte sich an wie Zähneziehen. Also haben wir letzten Frühling in den sauren Apfel gebissen und bei null angefangen.

## <a id="the-first-attempt-was-a-flop"></a>Der erste Versuch war ein Reinfall

Wir dachten, wir hätten das in drei Monaten im Griff. Spoiler: hatten wir nicht. Unser erster Prototyp war auf unseren Laptops rasend schnell und brach in dem Moment zusammen, als wir ihn auf echte Kundendaten losließen. Ordner mit 200.000 Dateien? Game over.

## <a id="what-finally-clicked"></a>Was schließlich den Durchbruch brachte

Der Durchbruch kam, als wir aufhörten, Dateien einzeln zu vergleichen, und stattdessen anfingen, Hashes ganzer Ordner zu vergleichen. Wenn der Hash übereinstimmt, überspringen wir den Ordner. Einfach, oder? Es hat viel zu lange gedauert, bis wir das erkannt haben.

## <a id="numbers-because-you-asked"></a>Zahlen, weil du gefragt hast

- Erster Scan von 1 Million Dateien: vorher 41 Minuten, jetzt 6 Minuten
- Arbeitsspeichernutzung: von 1,2 GB auf 300 MB gesunken
- Absturzberichte: um 80 % gesunken

## <a id="whats-next"></a>Was als Nächstes kommt

Wir sind noch nicht fertig. Als Nächstes kommt die Peer-to-Peer-Synchronisierung im lokalen Netzwerk, damit deine Dateien keinen Hin- und Rückweg durch die Cloud machen müssen, wenn dein Kollege direkt neben dir sitzt. Bleib dran, und wenn du auf einen Bug stößt, gib uns Bescheid!
