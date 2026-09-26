# <a id="rest-api-error-codes"></a>REST-API-Fehlercodes

Jede Fehlerantwort hat dieselbe Struktur:

```json
{ "error": { "code": "QuotaExceeded", "message": "The storage quota is exceeded." } }
```

| HTTP-Status | Code | Beschreibung | Erneut versuchen? |
|---|---|---|---|
| 400 | `InvalidPath` | Der Pfad enthält ungültige Zeichen | Nein |
| 401 | `Unauthorized` | Das Token fehlt oder ist abgelaufen | Nach der Anmeldung |
| 403 | `Forbidden` | Sie haben keinen Zugriff auf den Ordner | Nein |
| 404 | `NotFound` | Die Datei ist nicht vorhanden | Nein |
| 409 | `Conflict` | Die Datei wurde von einer anderen Person geändert | Ja, nach einer Aktualisierung |
| 413 | `TooLarge` | Die Datei ist größer als 250 GB | Nein |
| 423 | `Locked` | Die Datei ist ausgecheckt | Später |
| 429 | `TooManyRequests` | Sie haben zu viele Anfragen gesendet | Ja, nach `Retry-After` Sekunden |
| 507 | `QuotaExceeded` | Das Speicherkontingent wurde überschritten | Nein |

## <a id="handling-conflicts"></a>Konflikte behandeln

Wenn Sie `409 Conflict` erhalten, laden Sie die neueste Version herunter, führen Sie Ihre Änderungen zusammen, und laden Sie das Ergebnis mit dem neuen `ETag` im `If-Match`-Header erneut hoch.
