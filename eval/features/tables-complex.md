# Plan comparison

| Feature | Free | Team | Enterprise |
|:---|:---:|:---:|---:|
| Storage per user | 5 GB | 1 TB | Unlimited |
| **Version history** | 30 days | 180 days | Custom |
| Admin center | ✗ | ✓ | ✓ |
| [Audit log](#audit-log) | ✗ | 90 days | 10 years |
| API access (`/v2/*`) | Read only | Read and write | Read and write |
| Support | Community forum | E-mail, next business day | Phone, *24/7*, with a named contact |

## Audit log

| Event | Logged fields | Example |
|---|---|---|
| File shared | user, file, recipient | `alice@contoso.example` shared `plan.docx` |
| Link created | user, file, link type | Anonymous link, expires in 7 days |
| Sign-in failed | user, IP address, reason | Wrong password<br>Account locked |
| Setting changed | admin, setting, old value, new value | `retention`: 30 → 90 |

Empty cells and a single-column table:

| Notes |
|---|
| The Enterprise plan requires a yearly contract. |
| |
| Prices exclude VAT. |
