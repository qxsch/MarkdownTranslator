# <a id="deployment-checklist"></a>Liste de contrôle du déploiement

Effectuez les tâches suivantes avant un déploiement en production :

- Infrastructure
  - Vérifiez le quota dans la région cible :
    - Cœurs de processeur
    - Adresses IP publiques
    - Comptes de stockage
  - Vérifiez que les règles réseau autorisent :
    1. Le trafic sortant sur le port 443
    2. Le trafic entrant provenant uniquement de l’équilibreur de charge
- Application
  - Exécutez la suite de tests. Tous les éléments suivants doivent réussir :
    - Tests unitaires
    - Tests d’intégration, y compris les tests lents
  - Attribuez une étiquette à la version, par exemple `v3.2.0`
- Équipe
  - Informez l’équipe de support au moins un jour à l’avance
  - Désignez un ingénieur d’astreinte capable de :
    - Restaurer la version précédente
    - Contacter le client

Après le déploiement, vérifiez les éléments suivants :

1. Le point de terminaison d’intégrité `/healthz` retourne `200`.
2. Les taux d’erreur restent inférieurs au niveau de référence pendant 30 minutes.
3. Aucune alerte ne se déclenche.
   - Si une alerte se déclenche, suivez le runbook.
   - Si le runbook n’est d’aucune aide, restaurez la version précédente.
