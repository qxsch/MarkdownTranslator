# Docstrings

```python
def upload(path, retries=3):
    """Upload a file to the storage account.

    The upload is retried when the service is busy.

    Args:
        path (str): Local path of the file to upload.
        retries (int): How many times to retry.

    Returns:
        bool: True if the upload succeeded.

    Raises:
        ValueError: If the path does not exist.

    Examples:
        Upload a single file.

        >>> upload("data.csv")
        True

        Or use the command line::

            python upload.py data.csv --retries 5
            python upload.py other.csv

        Then check the result.

    .. note:: Large files are uploaded in blocks.
    """


def download(name):
    """Download a blob.

    Parameters
    ----------
    name : str
        Name of the blob to download.

    Returns
    -------
    bytes
        The content of the blob.

    Notes
    -----
    Uses ``Range`` requests for large blobs.
    """
```
