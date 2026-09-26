# <a id="python-sdk-reference-sharing"></a>Python-SDK-Referenz: Freigabe

```python
def share(path, recipients, role="viewer", expires=None):
    """Gibt eine Datei oder einen Ordner für andere Personen frei.

    Empfänger erhalten eine E-Mail mit einem Link. Externe Empfänger
    müssen ihre Adresse bestätigen, bevor sie den Link öffnen können.

    :param path: Pfad der Datei oder des Ordners, relativ zum Synchronisierungsstamm.
    :type path: str
    :param recipients: E-Mail-Adressen der Personen, für die die Freigabe erfolgen soll.
    :type recipients: list[str]
    :param role: Entweder ``"viewer"`` oder ``"editor"``.
    :param expires: Zeitpunkt, ab dem der Link nicht mehr funktioniert. ``None`` bedeutet nie.
    :returns: Der erstellte Freigabelink.
    :rtype: ShareLink
    :raises PermissionError: Wenn Sie nicht der Besitzer des Elements sind.

    .. note:: Links zu Ordnern umfassen alle Unterordner.

    .. warning:: Bearbeiter können Dateien
       für alle löschen.

    .. versionadded:: 3.1
    """
```
