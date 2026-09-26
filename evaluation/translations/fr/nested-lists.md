# <a id="deployment-checklist"></a>Liste de contrôle du déploiement

Effectuez les tâches suivantes avant un déploiement en production :

- Infrastructure
  - Vérifiez les quotas dans la région cible :
    - Cœurs de processeur
    - Adresses IP publiques
    - Comptes de stockage
  - Vérifiez que les règles réseau autorisent :
    1. Trafic sortant sur le port 443
    2. Trafic entrant provenant uniquement de l’équilibreur de charge
- Application
  - Exécutez la suite de tests. Tous les tests suivants doivent réussir :
    - Tests unitaires
    - Tests d’intégration, y compris les plus lents
  - Balisez la version, par exemple `v3.2.0`
- Intervenants
  - Informez l’équipe d’assistance au moins un jour à l’avance
  - Désignez un ingénieur d’astreinte qui peut :
    - Revenir à la version précédente
    - Contacter le client

Après le déploiement, vérifiez les éléments suivants :

1. Le point de terminaison d’intégrité `/healthz` renvoie `200`.
2. Les taux d’erreurs restent inférieurs à la référence pendant 30 minutes.
3. Aucune alerte ne se déclenche.
   - Si une alerte se déclenche, suivez le runbook.
   - Si le runbook ne résout pas le problème, revenez à la version précédente.
