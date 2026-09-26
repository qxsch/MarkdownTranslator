# <a id="python-sdk-reference-statistics"></a>Référence du SDK Python : statistiques

```python
def usage_report(share, start, end):
    """Calcule l’utilisation du stockage d’un partage.

    Renvoie une ligne par jour avec l’espace utilisé et le nombre de fichiers.

    Parameters
    ----------
    share : str
        Nom du partage.
    start : datetime.date
        Premier jour du rapport, inclus.
    end : datetime.date
        Dernier jour du rapport, inclus.

    Returns
    -------
    pandas.DataFrame
        Une table avec les colonnes ``date``, ``bytes`` et ``files``.

    Raises
    ------
    ValueError
        Si ``start`` est postérieur à ``end``.

    See Also
    --------
    quota_report : Compare l’utilisation avec le quota.

    Notes
    -----
    Les jours sans activité sont inclus avec les valeurs du jour précédent.

    Examples
    --------
    >>> df = usage_report("finance", date(2026, 8, 1), date(2026, 8, 7))
    >>> len(df)
    7
    """
```
