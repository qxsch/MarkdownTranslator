# <a id="python-sdk-internals"></a>Python-SDK-Interna

```python
"""Hilfsfunktionen für den lokalen Cache.

Der Cache speichert Block-Hashwerte, damit unveränderte Blöcke nicht erneut hochgeladen werden.
"""

import sqlite3


class BlockCache:
    """Eine persistente Zuordnung von Block-Hashwerten zu Uploadstatus."""

    def __init__(self, path):
        """Öffnet oder erstellt die Cache-Datenbank unter ``path``."""
        self._db = sqlite3.connect(path)

    def seen(self, digest):
        r"""Gibt True zurück, wenn der Block mit diesem Digest zuvor hochgeladen wurde.

        Der Digest ist eine Hexadezimalzeichenfolge wie ``a3f\x00``; Raw-String-Literale behalten den Backslash bei.
        """
        row = self._db.execute("SELECT 1 FROM blocks WHERE digest = ?", (digest,)).fetchone()
        return row is not None

    def clear(self):
        message = f"""Clearing {self._db} is not a docstring and must stay unchanged."""
        self._db.execute("DELETE FROM blocks")
        return message
```
