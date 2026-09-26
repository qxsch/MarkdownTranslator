# <a id="python-sdk-reference-statistics"></a>Python-SDK-Referenz: Statistik

```python
def usage_report(share, start, end):
    """Berechnet die Speichernutzung für eine Freigabe.

    Gibt eine Zeile pro Tag mit dem belegten Speicherplatz und der Anzahl der Dateien zurück.

    Parameters
    ----------
    share : str
        Name der Freigabe.
    start : datetime.date
        Erster Tag des Berichts, einschließlich dieses Tages.
    end : datetime.date
        Letzter Tag des Berichts, einschließlich dieses Tages.

    Returns
    -------
    pandas.DataFrame
        Eine Tabelle mit den Spalten ``date``, ``bytes`` und ``files``.

    Raises
    ------
    ValueError
        Wenn ``start`` nach ``end`` liegt.

    See Also
    --------
    quota_report : Vergleicht die Nutzung mit dem Kontingent.

    Notes
    -----
    Tage ohne Aktivität werden mit den Werten des vorherigen Tages einbezogen.

    Examples
    --------
    >>> df = usage_report("finance", date(2026, 8, 1), date(2026, 8, 7))
    >>> len(df)
    7
    """
```
