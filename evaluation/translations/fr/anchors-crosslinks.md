# <a id="administrator-guide"></a>Guide de l’administrateur

Ce guide contient de nombreux liens vers ses propres sections. Commencez par [Planifier le déploiement](#plan-the-rollout), puis lisez [Attribuer des licences](#assign-licenses).

## <a id="plan-the-rollout"></a>Planifier le déploiement

Déterminez les groupes qui recevront le client en premier. Consultez [Groupes pilotes](#pilot-groups) pour obtenir des recommandations et [Revenir en arrière](#roll-back) en cas de problème.

### <a id="pilot-groups"></a>Groupes pilotes

Commencez avec un groupe de 20 à 50 utilisateurs. Leurs commentaires vous aident à ajuster les [paramètres par défaut](#default-settings).

## <a id="assign-licenses"></a>Attribuer des licences

Les licences sont attribuées par utilisateur. L’attribution basée sur les groupes est décrite dans [Automatiser l’attribution](#automate-assignment).

### <a id="automate-assignment"></a>Automatiser l’attribution

Utilisez l’API d’administration pour attribuer des licences à partir de votre système RH. L’API est décrite dans la [référence des erreurs](api-errors-table.md#handling-conflicts).

## <a id="default-settings"></a>Paramètres par défaut

Déployez les paramètres par défaut avec votre outil de gestion des appareils. Les paramètres que les utilisateurs ne peuvent pas modifier sont marqués comme **verrouillés**.

## <a id="roll-back"></a>Revenir en arrière

Pour revenir en arrière, désinstallez le client et supprimez la stratégie. Les fichiers restent dans le cloud. Revenez au [début](#administrator-guide).
