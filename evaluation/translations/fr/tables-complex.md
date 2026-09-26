# <a id="plan-comparison"></a>Comparaison des plans

| Fonctionnalité | Free | Team | Enterprise |
|:---|:---:|:---:|---:|
| Stockage par utilisateur | 5 Go | 1 To | Illimité |
| **Historique des versions** | 30 jours | 180 jours | Personnalisé |
| Centre d’administration | ✗ | ✓ | ✓ |
| [Journal d’audit](#audit-log) | ✗ | 90 jours | 10 ans |
| Accès à l’API (`/v2/*`) | Lecture seule | Lecture et écriture | Lecture et écriture |
| Support | Forum communautaire | E-mail, jour ouvré suivant | Téléphone, *24 h/24, 7 j/7*, avec un contact désigné |

## <a id="audit-log"></a>Journal d’audit

| Événement | Champs journalisés | Exemple |
|---|---|---|
| Fichier partagé | utilisateur, fichier, destinataire | `alice@contoso.example` a partagé `plan.docx` |
| Lien créé | utilisateur, fichier, type de lien | Lien anonyme, expiration dans 7 jours |
| Échec de connexion | utilisateur, adresse IP, motif | Mot de passe incorrect<br>Compte verrouillé |
| Paramètre modifié | administrateur, paramètre, ancienne valeur, nouvelle valeur | `retention`: 30 → 90 |

Cellules vides et tableau à une seule colonne :

| Notes |
|---|
| Le plan Enterprise nécessite un contrat annuel. |
| |
| Les prix s’entendent hors TVA. |
