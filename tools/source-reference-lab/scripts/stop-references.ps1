$ErrorActionPreference = 'Stop'

$LabRoot = Split-Path -Parent $PSScriptRoot
$WorkspaceRoot = Resolve-Path (Join-Path $LabRoot '..\..')
$ProcessFile = Join-Path $WorkspaceRoot 'output\source-reference-lab\processes.json'

if (-not (Test-Path -LiteralPath $ProcessFile)) {
    Write-Host 'No process record found.'
    exit 0
}

$Records = Get-Content -LiteralPath $ProcessFile -Raw | ConvertFrom-Json
$AllowedPorts = @(4190, 4191, 4192, 4193, 4194)
$Connections = @(Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | Where-Object {
    $_.LocalAddress -eq '127.0.0.1' -and $_.LocalPort -in $AllowedPorts
})

foreach ($Connection in $Connections) {
    $TargetPid = [int]$Connection.OwningProcess
    $Process = Get-Process -Id $TargetPid -ErrorAction SilentlyContinue
    if ($null -eq $Process) { continue }
    if ($Process.ProcessName -ne 'node') {
        throw "Refusing to stop unexpected process $($Process.ProcessName) on reference port $($Connection.LocalPort)."
    }
    Stop-Process -Id $TargetPid
    Write-Host "Stopped reference server on $($Connection.LocalPort) (PID $TargetPid)"
}

Start-Sleep -Milliseconds 500

foreach ($Record in $Records) {
    $RecordedPid = [int]$Record.Id
    $Process = Get-Process -Id $RecordedPid -ErrorAction SilentlyContinue
    if ($null -ne $Process -and $Process.ProcessName -eq 'cmd') {
        Stop-Process -Id $RecordedPid
    }
}

Remove-Item -LiteralPath $ProcessFile
