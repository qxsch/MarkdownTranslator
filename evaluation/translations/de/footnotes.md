# <a id="performance-results"></a>Leistungsergebnisse

Der Synchronisierungsdurchsatz hat sich gegenüber dem vorherigen Release um 40 %[^1] verbessert. Für den Test wurde der Standardbenchmark[^bench] auf drei Computern verwendet.

Die Latenz blieb bei 99 % der Anfragen[^p99] unter 200 ms.

## <a id="method"></a>Methodik

Jeder Durchlauf begann mit einem leeren Cache.[^cache] Wir haben jede Messung fünfmal wiederholt und geben den Median an.

[^1]: Gemessen über eine 1-Gbit/s-Verbindung mit 10.000 Dateien von jeweils 1 MB.
[^bench]: Der Benchmark wird in `bench/README.md` beschrieben. Er erstellt Dateien mit zufälligem Inhalt.
[^p99]: Das 99. Perzentil, gemessen am Gateway.
[^cache]: Um den Cache zu leeren, führen Sie `ctsync reset --cache-only` aus. Ein zweiter Absatz in der
    Fußnote erklärt, warum ein kalter Cache wichtig ist: Der erste Scan liest jede Datei.
