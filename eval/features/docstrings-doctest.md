# Python SDK reference: paths

```python
def normalize(path):
    """Return the canonical form of a sync path.

    Backslashes become forward slashes and duplicate separators are
    removed.

    >>> normalize("docs\\\\reports//2026")
    'docs/reports/2026'
    >>> normalize("")
    ''

    Use it before you compare two paths::

        if normalize(a) == normalize(b):
            print("same file")

    The function never touches the file system.
    """


def is_hidden(name):
    '''Check whether a file name is hidden on the current platform.'''
    return name.startswith(".")
```
