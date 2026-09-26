# <a id="python-sdk-internals"></a>Détails internes du SDK Python

```python
"""Utilitaires pour le cache local.

Le cache stocke les hachages de blocs afin que les blocs inchangés ne soient pas téléversés à nouveau.
"""

import sqlite3


class BlockCache:
    """Mappage persistant du hachage de bloc vers l’état de téléversement."""

    def __init__(self, path):
        """Ouvre ou crée la base de données du cache à l’emplacement ``path``."""
        self._db = sqlite3.connect(path)

    def seen(self, digest):
        r"""Retourne True si le bloc avec ce digest a déjà été téléversé.

        Le digest est une chaîne hexadécimale telle que ``a3f\x00`` ; les chaînes brutes conservent la barre oblique inverse.
        """
        row = self._db.execute("SELECT 1 FROM blocks WHERE digest = ?", (digest,)).fetchone()
        return row is not None

    def clear(self):
        message = f"""Clearing {self._db} is not a docstring and must stay unchanged."""
        self._db.execute("DELETE FROM blocks")
        return message
```
