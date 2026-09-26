# Python SDK reference: statistics

```python
def usage_report(share, start, end):
    """Compute storage usage for a share.

    Returns one row per day with the used space and the number of files.

    Parameters
    ----------
    share : str
        Name of the share.
    start : datetime.date
        First day of the report, inclusive.
    end : datetime.date
        Last day of the report, inclusive.

    Returns
    -------
    pandas.DataFrame
        A table with the columns ``date``, ``bytes`` and ``files``.

    Raises
    ------
    ValueError
        If ``start`` is after ``end``.

    See Also
    --------
    quota_report : Compares usage with the quota.

    Notes
    -----
    Days without activity are included with the values of the previous day.

    Examples
    --------
    >>> df = usage_report("finance", date(2026, 8, 1), date(2026, 8, 7))
    >>> len(df)
    7
    """
```
