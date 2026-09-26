# <a id="capacity-planning"></a>Kapazitätsplanung

Ein einzelnes Gateway verarbeitet etwa $r = 400$ Anforderungen pro Sekunde. Für eine Spitzenlast von $R$ Anforderungen pro Sekunde benötigen Sie $n = \lceil R / r \rceil$ Gateways.

Jedes Gateway kostet 120 $ pro Monat. Für 2.000 Anforderungen pro Sekunde benötigen Sie 5 Gateways, was 600 $ pro Monat kostet.

Blockformel für die Speicherschätzung:

$$
S = u \cdot f \cdot (1 + v)
$$

wobei $u$ die Anzahl der Benutzer, $f$ die durchschnittliche Datenmenge pro Benutzer und $v$ der Anteil alter Versionen ist.

Bei 500 Benutzern, jeweils 20 GB, und $v = 0.3$ benötigen Sie 13 TB. Bei 20 $ pro TB kostet der Speicher 260 $ pro Monat.
