# <a id="python-sdk-reference-uploads"></a>Référence du SDK Python : chargements

```python
def upload(path, retries=3, overwrite=False):
    """Charge un fichier dans le partage de synchronisation.

    Le chargement est retenté avec une temporisation exponentielle
    lorsque le service renvoie une erreur transitoire.

    Args:
        path (str): Chemin local du fichier à charger.
        retries (int): Nombre de nouvelles tentatives pour un bloc en échec.
        overwrite (bool, optional): Remplace un fichier existant
            portant le même nom. Valeur par défaut : False.

    Returns:
        UploadResult: Détails sur le fichier chargé, y compris son
        numéro de version.

    Raises:
        FileNotFoundError: Si le fichier local n’existe pas.
        QuotaExceededError: Si le partage est plein.

    Example:
        Chargez un rapport et affichez sa version.

        >>> result = upload("report.pdf")
        >>> print(result.version)
        1
    """


class UploadResult:
    """Résultat d’un chargement.

    Attributes:
        version (int): Numéro de version attribué par le serveur.
        size (int): Taille du fichier chargé en octets.
    """
```
