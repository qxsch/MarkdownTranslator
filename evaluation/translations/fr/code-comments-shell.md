# <a id="scripts-for-administrators"></a>Scripts pour les administrateurs

Un script Bash qui vérifie chaque appareil :

```bash
#!/usr/bin/env bash
set -euo pipefail

# S’arrêter au premier appareil en échec
for host in $(cat hosts.txt); do
  ssh "$host" ctsync status --json > "status-$host.json"  # un fichier par appareil
done

# Afficher un résumé
jq -s 'map(.state) | group_by(.) | map({state: .[0], count: length})' status-*.json
```

La même vérification dans PowerShell :

```powershell
<#
.SYNOPSIS Collecte l’état de synchronisation de tous les appareils.
  .DESCRIPTION Lit les noms des appareils depuis hosts.txt et écrit un
  fichier JSON par appareil.
#>
param([string]$HostFile = 'hosts.txt')

# Collecter les résultats dans une liste
$results = foreach ($h in Get-Content $HostFile) {
    Invoke-Command -ComputerName $h { ctsync status --json }  # s’exécute à distance
}
$results | ConvertTo-Json | Set-Content summary.json
```

Un fichier de commandes Windows :

```bat
@echo off
REM Redémarrer le service de synchronisation
net stop ContosoSync
net start ContosoSync
:: Terminé
```
