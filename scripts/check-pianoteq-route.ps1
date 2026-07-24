param(
    [string]$PortName = "Neothesia to Pianoteq",
    [switch]$RequirePianoteq,
    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$repository = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

if (-not $SkipBuild) {
    & cargo build `
        -p neothesia `
        --bin midi-diagnostics `
        --manifest-path (Join-Path $repository "Cargo.toml")
    if ($LASTEXITCODE -ne 0) {
        throw "MIDI diagnostics build failed with exit code $LASTEXITCODE"
    }
}

$diagnostics = Join-Path $repository "target\debug\midi-diagnostics.exe"
if (-not (Test-Path -LiteralPath $diagnostics)) {
    throw "MIDI diagnostics executable is missing: $diagnostics"
}

Push-Location $repository
try {
    & $diagnostics `
        --expect-output $PortName `
        --require-saved-output `
        --probe-output
    if ($LASTEXITCODE -ne 0) {
        throw "Neothesia route validation failed with exit code $LASTEXITCODE"
    }
}
finally {
    Pop-Location
}

$pianoteq = @(
    Get-Process -ErrorAction SilentlyContinue |
        Where-Object { $_.ProcessName -like "Pianoteq*" }
)
$pianoteqRunning = $pianoteq.Count -gt 0

if ($RequirePianoteq -and -not $pianoteqRunning) {
    throw "The MIDI route is ready, but Pianoteq is not running."
}

[pscustomobject]@{
    RouteReady = $true
    Port = $PortName
    SavedSelectionMatches = $true
    OpenProbePassed = $true
    PianoteqRunning = $pianoteqRunning
    PhysicalAudioVerified = $false
}
