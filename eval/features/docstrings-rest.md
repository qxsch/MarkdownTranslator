# Python SDK reference: sharing

```python
def share(path, recipients, role="viewer", expires=None):
    """Share a file or folder with other people.

    Recipients get an e-mail with a link. External recipients must verify
    their address before they can open the link.

    :param path: Path of the file or folder, relative to the sync root.
    :type path: str
    :param recipients: E-mail addresses of the people to share with.
    :type recipients: list[str]
    :param role: Either ``"viewer"`` or ``"editor"``.
    :param expires: When the link stops working. ``None`` means never.
    :returns: The created sharing link.
    :rtype: ShareLink
    :raises PermissionError: If you aren't the owner of the item.

    .. note:: Links to folders include all subfolders.

    .. warning::
       Editors can delete files for everyone.

    .. versionadded:: 3.1
    """
```
