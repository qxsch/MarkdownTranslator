# Python SDK reference: uploads

```python
def upload(path, retries=3, overwrite=False):
    """Upload a file to the sync share.

    The upload is retried with exponential backoff when the service
    returns a transient error.

    Args:
        path (str): Local path of the file to upload.
        retries (int): How many times to retry a failed block.
        overwrite (bool, optional): Replace an existing file with the
            same name. Defaults to False.

    Returns:
        UploadResult: Details about the uploaded file, including its
        version number.

    Raises:
        FileNotFoundError: If the local file doesn't exist.
        QuotaExceededError: If the share is full.

    Example:
        Upload a report and print its version.

        >>> result = upload("report.pdf")
        >>> print(result.version)
        1
    """


class UploadResult:
    """Result of an upload.

    Attributes:
        version (int): Version number assigned by the server.
        size (int): Size of the uploaded file in bytes.
    """
```
