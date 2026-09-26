# <a id="security-baseline"></a>Base de référence de sécurité

## <a id="contents"></a>Sommaire

- [Vue d’ensemble](#overview)
- [Identité](#identity)
  - [Authentification multifacteur](#multifactor-authentication)
  - [Accès conditionnel](#conditional-access)
- [Protection des données](#data-protection)
  - [Chiffrement](#encryption)
  - [Liens de partage](#sharing-links)
- [Surveillance](#monitoring)

## <a id="overview"></a>Vue d’ensemble

Cette base de référence répertorie les paramètres de sécurité recommandés pour Contoso Sync.

## <a id="identity"></a>Identité

### <a id="multifactor-authentication"></a>Authentification multifacteur

Exigez l’authentification multifacteur pour tous les utilisateurs.

### <a id="conditional-access"></a>Accès conditionnel

Autorisez la connexion uniquement à partir d’appareils gérés.

## <a id="data-protection"></a>Protection des données

### <a id="encryption"></a>Chiffrement

Le chiffrement au repos est toujours activé. Les clés gérées par le client sont facultatives.

### <a id="sharing-links"></a>Liens de partage

Définissez le type de lien par défaut sur **Personnes dans votre organisation**. Les liens anonymes doivent expirer au bout de 7 jours.

## <a id="monitoring"></a>Surveillance

Envoyez le journal d’audit à votre SIEM. Passez en revue les [liens de partage](#sharing-links) tous les mois.
