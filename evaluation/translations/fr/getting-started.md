# <a id="getting-started-with-contoso-sync"></a>Bien démarrer avec Contoso Sync

Contoso Sync synchronise un dossier local et un partage cloud. Il faut environ dix minutes pour suivre ce guide.

## <a id="prerequisites"></a>Prérequis

Avant de commencer, vérifiez que vous disposez des éléments suivants :

- Un compte Contoso avec le rôle **Contributeur**
- Windows 11 ou macOS 14 ou version ultérieure
- Au moins 2 Go d’espace disque disponible

## <a id="install-the-client"></a>Installer le client

1. Téléchargez `ContosoSync-Setup.exe` à partir de la [page de téléchargement](https://contoso.example/download).
2. Exécutez le programme d’installation et sélectionnez **Suivant**.
3. Lorsque vous êtes invité à choisir un dossier, conservez le dossier par défaut `C:\Users\<you>\ContosoSync` ou sélectionnez **Parcourir**.
4. Sélectionnez **Installer**, puis **Terminer**.

> [!TIP]
> Vous pouvez installer le client en mode silencieux avec `ContosoSync-Setup.exe /quiet`.

## <a id="sign-in"></a>Se connecter

Ouvrez l’application à partir du menu Démarrer. Entrez votre adresse e-mail professionnelle et sélectionnez **Se connecter**. Si votre organisation utilise l’authentification multifacteur, approuvez la demande sur votre téléphone.

## <a id="choose-what-to-sync"></a>Choisir les éléments à synchroniser

| Option | Par défaut | Description |
|---|---|---|
| Fichiers à la demande | Activé | Les fichiers ne sont téléchargés que lorsque vous les ouvrez |
| Synchroniser sur les réseaux limités | Désactivé | Met en pause la synchronisation sur les points d’accès mobiles |
| Conserver | 30 jours | Durée pendant laquelle les fichiers supprimés restent dans la Corbeille |

## <a id="next-steps"></a>Étapes suivantes

- [Partager un dossier](#choose-what-to-sync) avec votre équipe
- Lisez le [guide de résolution des problèmes](troubleshooting.md)
