$ErrorActionPreference = 'Stop'

$LabRoot = Split-Path -Parent $PSScriptRoot
$WorkspaceRoot = Resolve-Path (Join-Path $LabRoot '..\..')
$GlaceRoot = Join-Path $WorkspaceRoot 'example\liquid glass\glace-main'
$GlaceSite = Join-Path $GlaceRoot 'site'
$WeatherRoot = Join-Path $WorkspaceRoot 'example\react-weather-effects-master'
$CloudRoot = Join-Path $WorkspaceRoot 'example\webgpu_realtime_clouds-main'

function Invoke-SetupStep {
    param(
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][string]$WorkingDirectory,
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )

    Write-Host "[$Label] $Executable $($Arguments -join ' ')"
    Push-Location -LiteralPath $WorkingDirectory
    try {
        & $Executable @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "$Label failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }
}

Invoke-SetupStep -Label 'Reference Lab' -WorkingDirectory $LabRoot -Executable 'pnpm.cmd' -Arguments @('install', '--frozen-lockfile=false')
Invoke-SetupStep -Label 'Glacé package' -WorkingDirectory $GlaceRoot -Executable 'npm.cmd' -Arguments @('ci')
Invoke-SetupStep -Label 'Glacé build' -WorkingDirectory $GlaceRoot -Executable 'npm.cmd' -Arguments @('run', 'build')
Invoke-SetupStep -Label 'Glacé site' -WorkingDirectory $GlaceSite -Executable 'npm.cmd' -Arguments @('install', '--no-audit', '--no-fund', '--prefer-offline')
Invoke-SetupStep -Label 'Glacé site build' -WorkingDirectory $GlaceSite -Executable 'npm.cmd' -Arguments @('run', 'build')
Invoke-SetupStep -Label 'Weather demo' -WorkingDirectory $WeatherRoot -Executable 'npm.cmd' -Arguments @('install', '--no-audit', '--no-fund', '--prefer-offline')
Invoke-SetupStep -Label 'Weather demo build' -WorkingDirectory $WeatherRoot -Executable 'npm.cmd' -Arguments @('run', 'build')
Invoke-SetupStep -Label 'Realtime Clouds build' -WorkingDirectory $CloudRoot -Executable 'npx.cmd' -Arguments @('--yes', 'tinybuild', 'build')

Write-Host 'Reference sources are ready.' -ForegroundColor Green
