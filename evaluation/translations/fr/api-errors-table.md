# <a id="rest-api-error-codes"></a>Codes d’erreur de l’API REST

Chaque réponse d’erreur a la même structure :

```json
{ "error": { "code": "QuotaExceeded", "message": "The storage quota is exceeded." } }
```

| État HTTP | Code | Description | Réessayer ? |
|---|---|---|---|
| 400 | `InvalidPath` | Le chemin contient des caractères non valides | Non |
| 401 | `Unauthorized` | Le token est manquant ou a expiré | Après la connexion |
| 403 | `Forbidden` | Vous n’avez pas accès au dossier | Non |
| 404 | `NotFound` | Le fichier n’existe pas | Non |
| 409 | `Conflict` | Le fichier a été modifié par quelqu’un d’autre | Oui, après une actualisation |
| 413 | `TooLarge` | La taille du fichier dépasse 250 Go | Non |
| 423 | `Locked` | Le fichier est extrait | Plus tard |
| 429 | `TooManyRequests` | Vous avez envoyé trop de requêtes | Oui, après `Retry-After` secondes |
| 507 | `QuotaExceeded` | Le quota de stockage est dépassé | Non |

## <a id="handling-conflicts"></a>Gestion des conflits

Lorsque vous recevez `409 Conflict`, téléchargez la dernière version, fusionnez vos modifications et chargez à nouveau le fichier avec le nouvel `ETag` dans l’en-tête `If-Match`.
