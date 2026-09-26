# REST API error codes

Every error response has the same shape:

```json
{ "error": { "code": "QuotaExceeded", "message": "The storage quota is exceeded." } }
```

| HTTP status | Code | Description | Retry? |
|---|---|---|---|
| 400 | `InvalidPath` | The path contains invalid characters | No |
| 401 | `Unauthorized` | The token is missing or expired | After sign-in |
| 403 | `Forbidden` | You don't have access to the folder | No |
| 404 | `NotFound` | The file doesn't exist | No |
| 409 | `Conflict` | The file was changed by someone else | Yes, after a refresh |
| 413 | `TooLarge` | The file is larger than 250 GB | No |
| 423 | `Locked` | The file is checked out | Later |
| 429 | `TooManyRequests` | You sent too many requests | Yes, after `Retry-After` seconds |
| 507 | `QuotaExceeded` | The storage quota is exceeded | No |

## Handling conflicts

When you get `409 Conflict`, download the latest version, merge your changes and upload again with the new `ETag` in the `If-Match` header.
