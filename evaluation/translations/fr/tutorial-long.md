# <a id="tutorial-automate-backups-with-contoso-sync"></a>Tutoriel : automatiser les sauvegardes avec Contoso Sync

Dans ce tutoriel, vous configurez une sauvegarde nocturne d’un dossier de projet, testez une restauration et recevez une notification en cas de problème.

## <a id="part-1-prepare-the-folder"></a>Partie 1 : préparer le dossier

Créez un dossier nommé `projects` dans votre répertoire de base. Déplacez-y les fichiers que vous souhaitez sauvegarder. Évitez les dossiers qui contiennent des sorties de build, tels que `node_modules` ou `bin`, car ils changent souvent et utilisent beaucoup d’espace.

Ensuite, ajoutez un fichier nommé `.ctsyncignore` pour indiquer les modèles d’exclusion :

```text
node_modules/
bin/
*.tmp
```

## <a id="part-2-create-the-backup-job"></a>Partie 2 : créer la tâche de sauvegarde

1. Ouvrez **Paramètres** > **Sauvegardes**.
2. Sélectionnez **Nouvelle tâche**.
3. Entrez un nom, par exemple *Projets nocturnes*.
4. Sous **Planification**, sélectionnez **Quotidienne** et définissez l’heure sur 02:00.
5. Sous **Conserver**, sélectionnez le nombre de versions à conserver. Sept constitue un bon point de départ.
6. Sélectionnez **Créer**.

La tâche apparaît dans la liste avec l’état **Planifiée**.

## <a id="part-3-test-a-restore"></a>Partie 3 : tester une restauration

Une sauvegarde n’est utile que si vous pouvez la restaurer. Supprimez un fichier de test, puis :

1. Sélectionnez la tâche, puis **Restaurer**.
2. Choisissez la version d’hier.
3. Choisissez **Restaurer à l’emplacement d’origine**.

Le fichier réapparaît en quelques secondes.

## <a id="part-4-get-notified"></a>Partie 4 : recevoir des notifications

Sous **Notifications**, activez **M’envoyer un e-mail en cas d’échec d’une tâche**. Vous pouvez également envoyer des événements à un webhook :

```json
{
  "webhook": "https://hooks.example.com/backup",
  "events": ["job.failed", "job.succeeded"]
}
```

## <a id="clean-up"></a>Nettoyer

Si vous ne souhaitez pas conserver la tâche, sélectionnez-la, puis **Supprimer**. La suppression de la tâche ne supprime pas les sauvegardes existantes ; elles expirent conformément au paramètre de rétention.

## <a id="summary"></a>Résumé

Vous avez créé une sauvegarde planifiée, testé une restauration et configuré des notifications. Continuez avec [Partager un dossier avec votre équipe](share.md).
