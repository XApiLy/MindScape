param(
    [string]$Label = "manual",
    [ValidateRange(1, 20)]
    [int]$Keep = 5
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$desktopRoot = Join-Path $repoRoot "desktop"
$acceptanceRoot = Join-Path $repoRoot "artifacts\acceptance"
$versionsRoot = Join-Path $acceptanceRoot "versions"

$buildStartedAt = Get-Date
Push-Location $desktopRoot
try {
    & pnpm tauri build --no-bundle
    if ($LASTEXITCODE -ne 0) {
        throw "Tauri acceptance build failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}

$sourcePath = [IO.Path]::GetFullPath((Join-Path $desktopRoot "src-tauri\target\release\mindscape-desktop.exe"))
$repoPrefix = $repoRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
if (-not $sourcePath.StartsWith($repoPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Source executable must be inside the MindScape repository."
}
if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) {
    throw "Source executable does not exist: $sourcePath"
}
if ((Get-Item -LiteralPath $sourcePath).LastWriteTime -lt $buildStartedAt.AddSeconds(-2)) {
    throw "Tauri reported success but did not produce or refresh the release executable."
}

$safeLabel = ($Label.ToLowerInvariant() -replace '[^a-z0-9._-]+', '-').Trim('-')
if ([string]::IsNullOrWhiteSpace($safeLabel)) {
    $safeLabel = "manual"
}

$commit = (& git -C $repoRoot rev-parse --short=12 HEAD).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "Unable to determine the Git commit."
}
$branch = ((& git -C $repoRoot branch --show-current) | Out-String).Trim()
if ([string]::IsNullOrWhiteSpace($branch)) {
    $branch = ((& git -C $repoRoot rev-parse --abbrev-ref HEAD) | Out-String).Trim()
}
$workingTreeStatus = (& git -C $repoRoot status --porcelain=v1 --untracked-files=all) -join "`n"
$isDirty = -not [string]::IsNullOrWhiteSpace($workingTreeStatus)
$dirtySuffix = if ($isDirty) { "-dirty" } else { "" }
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$buildId = "$timestamp-$safeLabel-$commit$dirtySuffix"
$versionRoot = Join-Path $versionsRoot $buildId

if (Test-Path -LiteralPath $versionRoot) {
    throw "Acceptance build already exists: $buildId"
}

New-Item -ItemType Directory -Path $versionRoot -Force | Out-Null
$destinationExe = Join-Path $versionRoot "mindscape-desktop.exe"
Copy-Item -LiteralPath $sourcePath -Destination $destinationExe

$sourceBuiltAt = (Get-Item -LiteralPath $sourcePath).LastWriteTime.ToString("yyyy-MM-ddTHH:mm:sszzz")
$hash = (Get-FileHash -LiteralPath $destinationExe -Algorithm SHA256).Hash
$size = (Get-Item -LiteralPath $destinationExe).Length
$producedAt = (Get-Date).ToString("yyyy-MM-ddTHH:mm:sszzz")
$sourceRelative = $sourcePath.Substring($repoPrefix.Length).Replace('\', '/')
$executableRelative = "artifacts/acceptance/versions/$buildId/mindscape-desktop.exe"

$manifest = [ordered]@{
    schemaVersion = 1
    buildId = $buildId
    product = "MindScape Desktop"
    buildMode = "tauri-release-no-bundle"
    sourceBuiltAt = $sourceBuiltAt
    producedAt = $producedAt
    gitCommit = $commit
    gitBranch = $branch
    sourceTreeDirty = $isDirty
    sourceExecutable = $sourceRelative
    executable = $executableRelative
    sizeBytes = $size
    sha256 = $hash
}

$manifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $versionRoot "manifest.json") -Encoding UTF8
"$hash *mindscape-desktop.exe" | Set-Content -LiteralPath (Join-Path $versionRoot "SHA256SUMS.txt") -Encoding ASCII

$latest = @(
    "Build ID: $buildId"
    "Executable: $executableRelative"
    "Source built at: $sourceBuiltAt"
    "Produced at: $producedAt"
    "Git commit: $commit"
    "Source tree dirty: $isDirty"
    "SHA-256: $hash"
) -join "`r`n"
$latest | Set-Content -LiteralPath (Join-Path $acceptanceRoot "LATEST.txt") -Encoding UTF8

$versionDirectories = @(Get-ChildItem -LiteralPath $versionsRoot -Directory | Sort-Object LastWriteTimeUtc -Descending)
foreach ($stale in ($versionDirectories | Select-Object -Skip $Keep)) {
    try {
        Remove-Item -LiteralPath $stale.FullName -Recurse -Force
    }
    catch {
        Write-Warning "Could not remove old acceptance build '$($stale.Name)'. It may still be running."
    }
}

Write-Host "Acceptance build published."
Write-Host "Build ID: $buildId"
Write-Host "Executable: $destinationExe"
Write-Host "SHA-256: $hash"
if ($isDirty) {
    Write-Warning "The acceptance build was published from a dirty working tree. The manifest records this fact."
}
