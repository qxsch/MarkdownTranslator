#Requires -Modules Az.Accounts
<#
.SYNOPSIS
  Runs the mdtranslator container locally and prints ready-to-use test commands.
.EXAMPLE
  ./run-container.ps1 -Port 8080
  ./run-container.ps1 -Port 9090 -Build
  ./run-container.ps1 -Build -NpmRegistry https://npm-proxy.example.com/npm/
#>
[CmdletBinding()]
param(
    [int]$Port = 8080,
    [string]$Image = 'mdtranslator:local',
    # Subscription holding the AI resource; defaults to the current Az context.
    [string]$Subscription,
    [string]$EnvFile = '.env',
    # Key clients must send in x-api-key; generated when omitted.
    [string]$ApiKey,
    [switch]$Build,
    # npm package proxy/mirror for the image build; empty uses the public npm registry.
    [string]$NpmRegistry = $env:NPM_REGISTRY
)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

if (-not (Test-Path $EnvFile)) { throw "$EnvFile not found. Copy .env.example to $EnvFile and fill in the endpoints." }

docker image inspect $Image *> $null
if ($Build -or $LASTEXITCODE -ne 0) {
    Write-Host "Building $Image ..." -ForegroundColor Cyan
    $buildArgs = @('build', '-t', $Image)
    if ($NpmRegistry) { $buildArgs += @('--build-arg', "NPM_REGISTRY=$NpmRegistry") }
    docker @buildArgs .
    if ($LASTEXITCODE -ne 0) { throw 'docker build failed' }
}

$envKey = (Get-Content $EnvFile | Where-Object { $_ -match '^AZURE_AI_API_KEY=.+' }) -replace '^AZURE_AI_API_KEY=', ''
$useToken = [string]::IsNullOrWhiteSpace($envKey)
if ($useToken) {
    # The container has no managed identity locally, so it gets a short-lived Entra token (about 1 hour).
    $ctx = Get-AzContext
    if ($Subscription -and $ctx -and $ctx.Subscription.Name -ne $Subscription -and $ctx.Subscription.Id -ne $Subscription) {
        $ctx = Set-AzContext -Subscription $Subscription
    }
    if (-not $ctx) { throw 'No Az context. Run Connect-AzAccount (and Set-AzContext) first, or set AZURE_AI_API_KEY in .env.' }
    $token = (Get-AzAccessToken -ResourceUrl 'https://cognitiveservices.azure.com' -TenantId $ctx.Tenant.Id).Token
    $env:AZURE_AI_ACCESS_TOKEN = if ($token -is [securestring]) { $token | ConvertFrom-SecureString -AsPlainText } else { $token }
}

if (-not $ApiKey) { $ApiKey = [guid]::NewGuid().ToString('N') }
$env:MDT_API_KEY = $ApiKey
# Read by send-file.ps1; removed when the container stops.
$keyFile = Join-Path $PSScriptRoot '.mdt-local-key'
Set-Content -Path $keyFile -Value $ApiKey -NoNewline
$base = "http://localhost:${Port}"

Write-Host @"

mdtranslator on $base   (auth to Azure AI: $(if ($useToken) { 'Entra token, valid ~1h' } else { 'API key from .env' }))

Test commands (run in another terminal):

  ./send-file.ps1 -Port $Port -InFile .\test\fixtures\identity.md -Target de -OutFile .\out\identity.de.md -Report

  `$h = @{ 'x-api-key' = '$ApiKey' }

  # health + languages
  Invoke-RestMethod $base/healthz
  (Invoke-RestMethod $base/languages -Headers `$h).languages | Format-Table

  # one inline document -> German
  (Invoke-RestMethod -Method Post $base/translate/de -Headers `$h -ContentType 'application/json' ``
     -Body '{"main.md":"# Hello\n\nOpen **config.json** and run ``npm start``."}').'main.md'

  # a file -> French and Swedish, with report
  `$body = @{ 'main.md' = (Get-Content .\test\fixtures\identity.md -Raw) } | ConvertTo-Json
  `$r = Invoke-RestMethod -Method Post "$base/translate?to=fr,sv&includeReport=true" -Headers `$h ``
     -ContentType 'application/json; charset=utf-8' -Body ([Text.Encoding]::UTF8.GetBytes(`$body)) -TimeoutSec 900
  `$r.translations.fr.'main.md'
  `$r.report | Select-Object file, language, formality, segments, reviewEdits, keptSource, anchorsAdded

Press Ctrl+C to stop the container.

"@ -ForegroundColor Green

try {
    docker run --rm -it -p "${Port}:8080" --env-file $EnvFile -e AZURE_AI_ACCESS_TOKEN -e MDT_API_KEY -e MDT_CACHE_DIR= $Image
}
finally {
    Remove-Item Env:AZURE_AI_ACCESS_TOKEN, Env:MDT_API_KEY -ErrorAction SilentlyContinue
    Remove-Item $keyFile -ErrorAction SilentlyContinue
}
