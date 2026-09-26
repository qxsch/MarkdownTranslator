# <a id="python-sdk-reference-paths"></a>Référence du SDK Python : chemins

```python
def normalize(path):
    """Retourne la forme canonique d’un chemin de synchronisation.

    Les barres obliques inverses deviennent des barres obliques et
    les séparateurs en double sont supprimés.

    >>> normalize("docs\\\\reports//2026")
    'docs/reports/2026'
    >>> normalize("")
    ''

    Utilisez cette fonction avant de comparer deux chemins ::

        if normalize(a) == normalize(b):
            print("same file")

    La fonction n’accède jamais au système de fichiers.
    """


def is_hidden(name):
    '''Vérifie si un nom de fichier est caché sur la plateforme actuelle.'''
    return name.startswith(".")
```
