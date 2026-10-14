param([switch]$NoBrowser)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repository = Split-Path $PSScriptRoot -Parent
$web = Join-Path $repository 'neothesia-web'
$logs = Join-Path $repository 'work/web-runtime'
[System.IO.Directory]::CreateDirectory($logs) | Out-Null
function Test-Endpoint([string]$Uri) {
    try { $null = Invoke-WebRequest -Uri $Uri -TimeoutSec 2 -UseBasicParsing; return $true } catch { return $false }
}
if (-not (Test-Endpoint 'http://127.0.0.1:32124/api/state')) {
    $service = Join-Path $repository 'target/debug/neothesia-service.exe'
    $serviceStamp = Join-Path $repository 'target/debug/neothesia-service.source-stamp'
    $buildService = -not (Test-Path -LiteralPath $service -PathType Leaf) -or
        -not (Test-Path -LiteralPath $serviceStamp -PathType Leaf)
    if (-not $buildService) {
        $builtAt = (Get-Item -LiteralPath $serviceStamp).LastWriteTimeUtc
        $serviceInputs = @('Cargo.toml', 'Cargo.lock')
        foreach ($crate in @('wgpu-jumpstart', 'neothesia-image', 'piano-layout', 'midi-file', 'midi-io', 'neothesia-core', 'neothesia-engine')) {
            $serviceInputs += "$crate/Cargo.toml"
        }
        foreach ($relative in $serviceInputs) {
            $inputFile = Join-Path $repository $relative
            if ((Test-Path -LiteralPath $inputFile -PathType Leaf) -and
                (Get-Item -LiteralPath $inputFile).LastWriteTimeUtc -gt $builtAt) {
                $buildService = $true
                break
            }
        }
        if (-not $buildService) {
            foreach ($relative in @('wgpu-jumpstart/src', 'neothesia-image/src', 'piano-layout/src', 'midi-file/src', 'midi-io/src', 'neothesia-core/src', 'neothesia-engine/src', 'neothesia-engine/assets')) {
                $sourceDirectory = Join-Path $repository $relative
                if (-not (Test-Path -LiteralPath $sourceDirectory -PathType Container)) { continue }
                $newer = Get-ChildItem -LiteralPath $sourceDirectory -Recurse -File |
                    Where-Object { $_.LastWriteTimeUtc -gt $builtAt } |
                    Select-Object -First 1
                if ($newer) {
                    $buildService = $true
                    break
                }
            }
        }
    }
    if ($buildService) {
        Write-Output '音乐服务源码已更新，正在生成当前版本。'
        Push-Location $repository
        try { & cargo build -p neothesia-engine --bin neothesia-service; if ($LASTEXITCODE) { throw '音乐引擎编译失败' } } finally { Pop-Location }
        if (-not (Test-Path -LiteralPath $service -PathType Leaf)) { throw '编译后没有找到音乐服务程序' }
        Set-Content -LiteralPath $serviceStamp -Value 'Music service build completed.' -Encoding Ascii
    }
    $process = Start-Process -FilePath $service -WorkingDirectory $repository -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $logs 'engine.log') -RedirectStandardError (Join-Path $logs 'engine-error.log')
    $process.Id | Set-Content -LiteralPath (Join-Path $logs 'engine.pid')
}
if (-not (Test-Endpoint 'http://127.0.0.1:5173')) {
    $vite = Join-Path $web 'node_modules/vite/bin/vite.js'
    if (-not (Test-Path -LiteralPath $vite)) {
        Push-Location $web
        try { & npm.cmd ci; if ($LASTEXITCODE) { throw '前端依赖准备失败' } } finally { Pop-Location }
    }
    $process = Start-Process -FilePath (Get-Command node.exe).Source -ArgumentList @('"' + $vite + '"','--host','127.0.0.1','--port','5173','--strictPort') -WorkingDirectory $web -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $logs 'web.log') -RedirectStandardError (Join-Path $logs 'web-error.log')
    $process.Id | Set-Content -LiteralPath (Join-Path $logs 'web.pid')
}
$deadline = [DateTime]::UtcNow.AddSeconds(30)
while (-not (Test-Endpoint 'http://127.0.0.1:5173') -or -not (Test-Endpoint 'http://127.0.0.1:32124/api/state')) {
    if ([DateTime]::UtcNow -gt $deadline) { throw "启动超时，请查看 $logs 中的日志" }
    Start-Sleep -Milliseconds 300
}
if (-not $NoBrowser) { Start-Process 'http://127.0.0.1:5173' }
Write-Output '练习室已启动：http://127.0.0.1:5173'
