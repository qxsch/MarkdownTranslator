# <a id="python-sdk-reference-sharing"></a>Référence du SDK Python : partage

```python
def share(path, recipients, role="viewer", expires=None):
    """Partager un fichier ou un dossier avec d’autres personnes.

    Les destinataires reçoivent un e-mail contenant un lien. Les
    destinataires externes doivent vérifier leur adresse avant de pouvoir
    ouvrir le lien.

    :param path: Chemin du fichier ou du dossier, relatif à la racine de synchronisation.
    :type path: str
    :param recipients: Adresses e-mail des personnes avec lesquelles effectuer le partage.
    :type recipients: list[str]
    :param role: Soit ``"viewer"``, soit ``"editor"``.
    :param expires: Quand le lien cesse de fonctionner. ``None`` signifie « jamais ».
    :returns: Le lien de partage créé.
    :rtype: ShareLink
    :raises PermissionError: Si vous n’êtes pas le propriétaire de l’élément.

    .. note:: Les liens vers des dossiers incluent tous les sous-dossiers.

    .. warning:: Les personnes ayant le rôle
       d’éditeur peuvent supprimer des
       fichiers pour tout le monde.

    .. versionadded:: 3.1
    """
```
