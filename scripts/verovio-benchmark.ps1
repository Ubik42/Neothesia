param(
    [string]$TestRoot = "D:\Music\_tools\_test\neothesia-verovio"
)

$ErrorActionPreference = "Stop"
$revision = "45ece8fa9f4ff1b51a6dfe29515e18530a64b1aa"
$sourceRoot = "https://raw.githubusercontent.com/opensheetmusicdisplay/opensheetmusicdisplay/$revision/test/data"
$corpusRoot = Join-Path $env:TEMP "neothesia-musicxml-corpus-$revision"
$packageRoot = Join-Path $TestRoot "node_modules\verovio"
$dataRoot = Join-Path $TestRoot "data"
$svgRoot = Join-Path $dataRoot "svg"
$reportPath = Join-Path $dataRoot "verovio-benchmark.json"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

$fixtures = @(
    @{ Name = "JohannSebastianBach_PraeludiumInCDur_BWV846_1.xml"; Sha256 = "8681e72dc9712a8ea4555135d5752d10ebfb44e84d2f7046434626be10ba43a4" },
    @{ Name = "MuzioClementi_SonatinaOpus36No1_Part1.xml"; Sha256 = "fb2b3feff9e6e26357a30de4a507651023e77c5cc545f5e22effba51b06d56c0" },
    @{ Name = "MuzioClementi_SonatinaOpus36No1_Part2.xml"; Sha256 = "034e08e23c10b42b840280e3ed299c61cd8be120333701a4c3c6d2cc97a23cd4" },
    @{ Name = "OSMD_Function_Test_Pedals.musicxml"; Sha256 = "96e0ee21d66e868a96eb809ffad91c8ded02e938a9f3a84b195fcf6f231125eb" },
    @{ Name = "OSMD_Function_Test_Voice_Alignment.musicxml"; Sha256 = "052aa1ea9cc9061a580f418515385e4d127ee97ec027270de058bc93f892c157" },
    @{ Name = "OSMD_function_test_GraceNotes.xml"; Sha256 = "96c1fe37af8dabfbe5fb75330a3842746c408053f37909f42993438d92710f0e" },
    @{ Name = "BrookeWestSample.mxl"; Sha256 = "1f9e2ea84df1edf9ff713232e705589979025d21cebf7d735951d82954c6be01" },
    @{ Name = "OSMD_function_test_all.xml"; Sha256 = "e7576216add597943288ae08075de96bd58e80c56c484b1d74457baef33cdc09" }
)

if (-not (Test-Path -LiteralPath $packageRoot)) {
    throw "Verovio is not installed in $TestRoot. Run npm install there first."
}

New-Item -ItemType Directory -Path $corpusRoot, $dataRoot, $svgRoot -Force | Out-Null
$paths = foreach ($fixture in $fixtures) {
    $fixturePath = Join-Path $corpusRoot $fixture.Name
    $valid = Test-Path -LiteralPath $fixturePath
    if ($valid) {
        $valid = (Get-FileHash -LiteralPath $fixturePath -Algorithm SHA256).Hash.ToLowerInvariant() -eq $fixture.Sha256
    }
    if (-not $valid) {
        Invoke-WebRequest -Uri "$sourceRoot/$($fixture.Name)" -OutFile $fixturePath
    }
    $actual = (Get-FileHash -LiteralPath $fixturePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $fixture.Sha256) {
        throw "SHA-256 mismatch for $($fixture.Name): $actual"
    }
    $fixturePath
}

$env:VEROVIO_PACKAGE_ROOT = $packageRoot
$env:VEROVIO_SVG_OUTPUT_ROOT = $svgRoot
$env:VEROVIO_REPORT_PATH = $reportPath
try {
    & node (Join-Path $PSScriptRoot "verovio-benchmark.mjs") $paths
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Remove-Item Env:VEROVIO_PACKAGE_ROOT -ErrorAction SilentlyContinue
    Remove-Item Env:VEROVIO_SVG_OUTPUT_ROOT -ErrorAction SilentlyContinue
    Remove-Item Env:VEROVIO_REPORT_PATH -ErrorAction SilentlyContinue
}

$validationArguments = foreach ($fixture in $fixtures) {
    Join-Path $svgRoot "$([IO.Path]::GetFileNameWithoutExtension($fixture.Name)).manifest.json"
    $fixture.Sha256
}
Push-Location $repoRoot
try {
    & cargo run -q -p neothesia-core --example verovio-manifest-inspect -- $validationArguments
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}

Write-Host "Report: $reportPath"
Write-Host "SVG pages: $svgRoot"
