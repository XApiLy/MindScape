$ErrorActionPreference = 'Stop'

$LabRoot = Split-Path -Parent $PSScriptRoot
$WorkspaceRoot = Resolve-Path (Join-Path $LabRoot '..\..')
$GlaceSite = Join-Path $WorkspaceRoot 'example\liquid glass\glace-main\site'
$YbouaneRoot = Join-Path $WorkspaceRoot 'example\liquid glass\liquidglass-main'
$WeatherRoot = Join-Path $WorkspaceRoot 'example\react-weather-effects-master'
$CloudRoot = Join-Path $WorkspaceRoot 'example\webgpu_realtime_clouds-main'
$OutputRoot = Join-Path $WorkspaceRoot 'output\source-reference-lab'
$ProcessFile = Join-Path $OutputRoot 'processes.json'

New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null

function Test-PortListening {
    param([Parameter(Mandatory = $true)][int]$Port)
    return $null -ne (Get-NetTCPConnection -State Listen -LocalPort $Port -ErrorAction SilentlyContinue | Select-Object -First 1)
}

function Start-ReferenceProcess {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][int]$Port,
        [Parameter(Mandatory = $true)][string]$WorkingDirectory,
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )

    if (Test-PortListening -Port $Port) {
        Write-Host "[$Name] already listening on $Port"
        return $null
    }

    $safeName = $Name -replace '[^a-zA-Z0-9-]', '-'
    $stdout = Join-Path $OutputRoot "$safeName.stdout.log"
    $stderr = Join-Path $OutputRoot "$safeName.stderr.log"
    $process = Start-Process -FilePath $Executable -ArgumentList $Arguments -WorkingDirectory $WorkingDirectory -WindowStyle Hidden -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
    Write-Host "[$Name] starting on $Port (PID $($process.Id))"
    return [pscustomobject]@{ Name = $Name; Port = $Port; Id = $process.Id }
}

$Processes = @()
$Processes += Start-ReferenceProcess -Name 'reference-lab' -Port 4190 -WorkingDirectory $LabRoot -Executable 'pnpm.cmd' -Arguments @('dev')
$Processes += Start-ReferenceProcess -Name 'glace' -Port 4191 -WorkingDirectory $GlaceSite -Executable 'npm.cmd' -Arguments @('run', 'dev', '--', '--host', '127.0.0.1', '--port', '4191')
$Processes += Start-ReferenceProcess -Name 'ybouane' -Port 4192 -WorkingDirectory $YbouaneRoot -Executable 'npx.cmd' -Arguments @('--yes', 'vite', 'site', '--host', '127.0.0.1', '--port', '4192', '--strictPort')
$Processes += Start-ReferenceProcess -Name 'weather' -Port 4193 -WorkingDirectory $WeatherRoot -Executable 'npm.cmd' -Arguments @('run', 'dev', '--', '--hostname', '127.0.0.1', '--port', '4193')
$Processes += Start-ReferenceProcess -Name 'realtime-clouds' -Port 4194 -WorkingDirectory $CloudRoot -Executable 'npx.cmd' -Arguments @('--yes', 'vite', '.', '--host', '127.0.0.1', '--port', '4194', '--strictPort')

$Processes = @($Processes | Where-Object { $null -ne $_ })
if ($Processes.Count -gt 0) {
    $Processes | ConvertTo-Json | Set-Content -LiteralPath $ProcessFile -Encoding utf8
}

$Deadline = (Get-Date).AddSeconds(45)
do {
    $MissingPorts = @(4190, 4191, 4192, 4193, 4194 | Where-Object { -not (Test-PortListening -Port $_) })
    if ($MissingPorts.Count -eq 0) { break }
    Start-Sleep -Milliseconds 400
} while ((Get-Date) -lt $Deadline)

if ($MissingPorts.Count -gt 0) {
    Write-Warning "Still waiting for ports: $($MissingPorts -join ', '). Check $OutputRoot for logs."
}
else {
    Write-Host 'All reference pages are ready: http://127.0.0.1:4190/' -ForegroundColor Green
}
