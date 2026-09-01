$ErrorActionPreference = 'Stop'

$ToolRoot = Split-Path -Parent $PSScriptRoot
$WorkspaceRoot = Resolve-Path (Join-Path $ToolRoot '..\..')
$OutputRoot = Join-Path $WorkspaceRoot 'output\selected-visual-baseline'
$ProcessFile = Join-Path $OutputRoot 'process.json'
$Port = 4200

New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null

$Existing = Get-NetTCPConnection -State Listen -LocalPort $Port -ErrorAction SilentlyContinue | Select-Object -First 1
if ($null -ne $Existing) {
    Write-Host "Selected visual baseline is already running: http://127.0.0.1:$Port/"
    exit 0
}

$Process = Start-Process `
    -FilePath 'pnpm.cmd' `
    -ArgumentList @('dev') `
    -WorkingDirectory $ToolRoot `
    -WindowStyle Hidden `
    -RedirectStandardOutput (Join-Path $OutputRoot 'stdout.log') `
    -RedirectStandardError (Join-Path $OutputRoot 'stderr.log') `
    -PassThru

[pscustomobject]@{ Id = $Process.Id; Port = $Port } | ConvertTo-Json | Set-Content -LiteralPath $ProcessFile -Encoding utf8

$Deadline = (Get-Date).AddSeconds(30)
do {
    $Listener = Get-NetTCPConnection -State Listen -LocalPort $Port -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -ne $Listener) { break }
    Start-Sleep -Milliseconds 400
} while ((Get-Date) -lt $Deadline)

if ($null -eq $Listener) {
    throw "Selected visual baseline did not start. Check $OutputRoot."
}

Write-Host "Selected visual baseline ready: http://127.0.0.1:$Port/" -ForegroundColor Green
