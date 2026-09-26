# <a id="python-sdk-reference-paths"></a>Python-SDK-Referenz: Pfade

```python
def normalize(path):
    """Gibt die kanonische Form eines Synchronisierungspfads zurück.

    Umgekehrte Schrägstriche werden in Schrägstriche umgewandelt,
    und doppelte Trennzeichen werden entfernt.

    >>> normalize("docs\\\\reports//2026")
    'docs/reports/2026'
    >>> normalize("")
    ''

    Verwenden Sie die Funktion, bevor Sie zwei Pfade vergleichen::

        if normalize(a) == normalize(b):
            print("same file")

    Die Funktion greift niemals auf das Dateisystem zu.
    """


def is_hidden(name):
    '''Prüft, ob ein Dateiname auf der aktuellen Plattform ausgeblendet ist.'''
    return name.startswith(".")
```
