# <a id="using-the-python-sdk"></a>Utilisation du SDK Python

```python
import os
from contoso_sync import SyncClient

# Lire le point de terminaison depuis les variables d’environnement afin que le script fonctionne dans chaque environnement
client = SyncClient(endpoint=os.environ["CTSYNC_ENDPOINT"])

# Charger chaque fichier CSV dans le dossier actuel
for name in os.listdir("."):
    if name.endswith(".csv"):  # ignorer tout le reste
        client.upload(name)

# Attendre que le serveur ait traité tous les
# chargements. Cela peut prendre quelques minutes
# pour les fichiers volumineux.
client.wait()
```

Gérez explicitement les erreurs :

```python
try:
    client.upload("big.iso")
except QuotaExceededError:
    # Le stockage est plein : informez l’utilisateur au lieu de réessayer
    print("Storage quota exceeded")
```
