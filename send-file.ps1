<#
.SYNOPSIS
  Sends one Markdown file to the local mdtranslator container and writes the translation.
.EXAMPLE
  ./send-file.ps1 -Port 8080 -InFile .\docs\guide.md -Target de -OutFile .\docs\guide.de.md
  ./send-file.ps1 -InFile .\README.md -Target fr -Report
#>
[CmdletBinding()]
param(
    [int]$Port = 8080,
    [Parameter(Mandatory)][string]$InFile,
    [Parameter(Mandatory)][string]$Target,
    [string]$OutFile,
    # Defaults to $env:MDT_API_KEY, then the key written by run-container.ps1.
    [string]$ApiKey,
    [ValidateSet('formal', 'informal')][string]$Formality,
    [switch]$NoReview,
    [switch]$Report,
    [int]$TimeoutSec = 900
)

$ErrorActionPreference = 'Stop'

$inPath = (Resolve-Path $InFile).Path
if (-not $OutFile) {
    $OutFile = Join-Path (Split-Path $inPath) ("{0}.{1}{2}" -f [IO.Path]::GetFileNameWithoutExtension($inPath), $Target, [IO.Path]::GetExtension($inPath))
}

if (-not $ApiKey) { $ApiKey = $env:MDT_API_KEY }
$keyFile = Join-Path $PSScriptRoot '.mdt-local-key'
if (-not $ApiKey -and (Test-Path $keyFile)) { $ApiKey = (Get-Content $keyFile -Raw).Trim() }

# Decode without stripping a BOM so the output stays byte-identical outside translated text.
$text = [Text.Encoding]::UTF8.GetString([IO.File]::ReadAllBytes($inPath))
$name = Split-Path $inPath -Leaf
$body = [Text.Encoding]::UTF8.GetBytes((@{ $name = $text } | ConvertTo-Json -Compress))

$query = @()
if ($Formality) { $query += "formality=$Formality" }
if ($NoReview) { $query += 'review=false' }
if ($Report) { $query += 'includeReport=true' }
$uri = "http://localhost:${Port}/translate/$([uri]::EscapeDataString($Target))" + $(if ($query) { '?' + ($query -join '&') } else { '' })

$headers = @{}
if ($ApiKey) { $headers['x-api-key'] = $ApiKey }

Write-Host "Translating $name -> $Target via $uri ..." -ForegroundColor Cyan
$sw = [Diagnostics.Stopwatch]::StartNew()
try {
    $res = Invoke-WebRequest -Method Post -Uri $uri -Headers $headers -ContentType 'application/json; charset=utf-8' -Body $body -TimeoutSec $TimeoutSec
}
catch {
    $detail = $_.ErrorDetails.Message
    throw "Request failed: $($_.Exception.Message) $detail"
}
$json = [Text.Encoding]::UTF8.GetString($res.RawContentStream.ToArray()) | ConvertFrom-Json -AsHashtable
$files = if ($Report) { $json['files'] } else { $json }

$outDir = Split-Path ([IO.Path]::GetFullPath($OutFile))
if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Path $outDir | Out-Null }
[IO.File]::WriteAllText([IO.Path]::GetFullPath($OutFile), $files[$name], [Text.UTF8Encoding]::new($false))

Write-Host ("Done in {0:N1}s -> {1}" -f $sw.Elapsed.TotalSeconds, $OutFile) -ForegroundColor Green
if ($Report) {
    $json['report'] | ForEach-Object {
        [pscustomobject]@{
            language      = $_.language
            formality     = $_.formality
            segments      = $_.segments
            via           = ($_.via.GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join ' '
            reviewEdits   = $_.reviewEdits
            keptSource    = @($_.keptSource).Count
            reverted      = @($_.revertedForStructure).Count
            anchorsAdded  = $_.anchorsAdded -join ', '
            error         = $_.error
        }
    } | Format-List
}
