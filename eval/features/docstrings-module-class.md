# Python SDK internals

```python
"""Helpers for the local cache.

The cache stores block hashes so that unchanged blocks are not uploaded again.
"""

import sqlite3


class BlockCache:
    """A persistent map from block hash to upload state."""

    def __init__(self, path):
        """Open or create the cache database at ``path``."""
        self._db = sqlite3.connect(path)

    def seen(self, digest):
        r"""Return True if the block with this digest was uploaded before.

        The digest is a hex string such as ``a3f\x00``; raw strings keep the backslash.
        """
        row = self._db.execute("SELECT 1 FROM blocks WHERE digest = ?", (digest,)).fetchone()
        return row is not None

    def clear(self):
        message = f"""Clearing {self._db} is not a docstring and must stay unchanged."""
        self._db.execute("DELETE FROM blocks")
        return message
```
