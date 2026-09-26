# <a id="scripts-for-administrators"></a>Skripts für Administratoren

Ein Bash-Skript, das jedes Gerät prüft:

```bash
#!/usr/bin/env bash
set -euo pipefail

# Beim ersten Gerät, das fehlschlägt, beenden
for host in $(cat hosts.txt); do
  ssh "$host" ctsync status --json > "status-$host.json"  # eine Datei pro Gerät
done

# Zusammenfassung ausgeben
jq -s 'map(.state) | group_by(.) | map({state: .[0], count: length})' status-*.json
```

Dieselbe Prüfung in PowerShell:

```powershell
<#
.SYNOPSIS Erfasst den Synchronisierungsstatus aller Geräte. .DESCRIPTION
  Liest die Gerätenamen aus hosts.txt und schreibt eine JSON-Datei pro
  Gerät.
#>
param([string]$HostFile = 'hosts.txt')

# Ergebnisse in einer Liste erfassen
$results = foreach ($h in Get-Content $HostFile) {
    Invoke-Command -ComputerName $h { ctsync status --json }  # wird remote ausgeführt
}
$results | ConvertTo-Json | Set-Content summary.json
```

Eine Windows-Batchdatei:

```bat
@echo off
REM Synchronisierungsdienst neu starten
net stop ContosoSync
net start ContosoSync
:: Fertig
```
