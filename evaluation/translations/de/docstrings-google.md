# <a id="python-sdk-reference-uploads"></a>Python-SDK-Referenz: Uploads

```python
def upload(path, retries=3, overwrite=False):
    """Lädt eine Datei in die Synchronisierungsfreigabe hoch.

    Der Upload wird mit exponentiellem Backoff wiederholt, wenn der
    Dienst einen vorübergehenden Fehler zurückgibt.

    Args:
        path (str): Lokaler Pfad der hochzuladenden Datei.
        retries (int): Anzahl der Wiederholungsversuche für einen fehlgeschlagenen Block.
        overwrite (bool, optional): Ersetzt eine vorhandene Datei mit
            demselben Namen. Standardwert ist False.

    Returns:
        UploadResult: Details zur hochgeladenen Datei,
        einschließlich ihrer Versionsnummer.

    Raises:
        FileNotFoundError: Wenn die lokale Datei nicht vorhanden ist.
        QuotaExceededError: Wenn die Freigabe voll ist.

    Example:
        Laden Sie einen Bericht hoch, und geben Sie seine Version aus.

        >>> result = upload("report.pdf")
        >>> print(result.version)
        1
    """


class UploadResult:
    """Ergebnis eines Uploads.

    Attributes:
        version (int): Vom Server zugewiesene Versionsnummer.
        size (int): Größe der hochgeladenen Datei in Byte.
    """
```
