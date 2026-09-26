# Scripts for administrators

A Bash script that checks every device:

```bash
#!/usr/bin/env bash
set -euo pipefail

# Stop on the first device that fails
for host in $(cat hosts.txt); do
  ssh "$host" ctsync status --json > "status-$host.json"  # one file per device
done

# Print a summary
jq -s 'map(.state) | group_by(.) | map({state: .[0], count: length})' status-*.json
```

The same check in PowerShell:

```powershell
<#
.SYNOPSIS
  Collects the sync status from all devices.
.DESCRIPTION
  Reads the device names from hosts.txt and writes one JSON file per device.
#>
param([string]$HostFile = 'hosts.txt')

# Collect the results in a list
$results = foreach ($h in Get-Content $HostFile) {
    Invoke-Command -ComputerName $h { ctsync status --json }  # runs remotely
}
$results | ConvertTo-Json | Set-Content summary.json
```

A Windows batch file:

```bat
@echo off
REM Restart the sync service
net stop ContosoSync
net start ContosoSync
:: Done
```
