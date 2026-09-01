$ErrorActionPreference = 'Stop'

$Port = 4200
$Listener = Get-NetTCPConnection -State Listen -LocalPort $Port -ErrorAction SilentlyContinue | Select-Object -First 1
if ($null -eq $Listener) {
    Write-Host 'Selected visual baseline is not running.'
    exit 0
}

$TargetPid = [int]$Listener.OwningProcess
$Process = Get-Process -Id $TargetPid -ErrorAction SilentlyContinue
if ($null -eq $Process) { exit 0 }
if ($Process.ProcessName -ne 'node') {
    throw "Refusing to stop unexpected process $($Process.ProcessName) on port $Port."
}

Stop-Process -Id $TargetPid
Write-Host "Stopped selected visual baseline on port $Port (PID $TargetPid)."
