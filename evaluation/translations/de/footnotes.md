# <a id="performance-results"></a>Leistungsergebnisse

Der Synchronisierungsdurchsatz verbesserte sich gegenüber der vorherigen Version um 40 %[^1]. Im Test wurde der Standard-Benchmark[^bench] auf drei Computern verwendet.

Die Latenz blieb bei 99 % der Anfragen[^p99] unter 200 ms.

## <a id="method"></a>Methode

Jeder Durchlauf begann mit einem leeren Cache.[^cache] Wir haben jede Messung fünfmal wiederholt und geben den Median an.

[^1]: Gemessen bei einer 1-Gbit/s-Verbindung mit 10.000 Dateien von jeweils 1 MB.
[^bench]: Der Benchmark ist in `bench/README.md` beschrieben. Er erstellt Dateien mit zufälligem Inhalt.
[^p99]: Das 99. Perzentil, gemessen am Gateway.
[^cache]: Um den Cache zu leeren, führen Sie `ctsync reset --cache-only` aus. Ein zweiter Absatz in der
    Fußnote erläutert, warum ein kalter Cache wichtig ist: Beim ersten Scan wird jede Datei gelesen.
