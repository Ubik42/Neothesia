$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$web = Join-Path $root 'neothesia-web'
$binary = Join-Path $root 'target\debug\neothesia-desktop.exe'
$stamp = Join-Path $root 'target\debug\neothesia-desktop.source-stamp'

# Keep this list to the desktop app and its local Rust dependencies. Do not scan
# downloaded libraries, user songs, node_modules, or target build output.
$sourceDirectories = @(
    'wgpu-jumpstart\src', 'neothesia-image\src', 'piano-layout\src',
    'midi-file\src', 'midi-io\src', 'neothesia-core\src',
    'neothesia-engine\src', 'neothesia-engine\assets',
    'neothesia-web\src', 'neothesia-web\public',
    'neothesia-web\src-tauri\src', 'neothesia-web\src-tauri\icons',
    'neothesia-web\src-tauri\capabilities'
)
$sourceFiles = @(
    'Cargo.toml', 'Cargo.lock', 'default.sf2',
    'neothesia-web\package.json', 'neothesia-web\package-lock.json',
    'neothesia-web\index.html', 'neothesia-web\vite.config.ts',
    'neothesia-web\tsconfig.json',
    'neothesia-web\src-tauri\Cargo.toml',
    'neothesia-web\src-tauri\build.rs',
    'neothesia-web\src-tauri\tauri.conf.json'
)
$sourceFiles += @('wgpu-jumpstart', 'neothesia-image', 'piano-layout',
    'midi-file', 'midi-io', 'neothesia-core', 'neothesia-engine') |
    ForEach-Object { Join-Path $_ 'Cargo.toml' }

try {
    if (-not (Test-Path -LiteralPath $web -PathType Container)) {
        throw '没有找到 neothesia-web 源码目录。'
    }
    $needsBuild = -not (Test-Path -LiteralPath $binary -PathType Leaf) -or
        -not (Test-Path -LiteralPath $stamp -PathType Leaf)
    if (-not $needsBuild) {
        $builtAt = (Get-Item -LiteralPath $stamp).LastWriteTimeUtc
        foreach ($relative in $sourceFiles) {
            $path = Join-Path $root $relative
            if (-not (Test-Path -LiteralPath $path -PathType Leaf) -or
                (Get-Item -LiteralPath $path).LastWriteTimeUtc -gt $builtAt) {
                $needsBuild = $true
                break
            }
        }
        if (-not $needsBuild) {
            foreach ($relative in $sourceDirectories) {
                $path = Join-Path $root $relative
                if (-not (Test-Path -LiteralPath $path -PathType Container)) {
                    $needsBuild = $true
                    break
                }
                $newer = Get-Item -LiteralPath $path
                if ($newer.LastWriteTimeUtc -le $builtAt) {
                    $newer = Get-ChildItem -LiteralPath $path -Recurse -Force |
                    Where-Object { $_.LastWriteTimeUtc -gt $builtAt } |
                    Select-Object -First 1
                }
                if ($newer) {
                    $needsBuild = $true
                    break
                }
            }
        }
    }
    if (-not $needsBuild) {
        Write-Host 'Neothesia 桌面程序已经与源码同步，正在打开。'
        exit 0
    }
    Write-Host 'Neothesia 源码已有更新，正在生成当前桌面程序。首次构建可能需要几分钟。'
    Push-Location $web
    try {
        if (-not (Test-Path -LiteralPath (Join-Path $web 'node_modules\.bin\tauri.cmd') -PathType Leaf)) {
            Write-Host '正在安装桌面程序依赖。'
            & npm.cmd ci --no-audit --no-fund
            if ($LASTEXITCODE -ne 0) { throw "依赖安装失败，退出码：$LASTEXITCODE" }
        }
        & npm.cmd run desktop:build:dev
        if ($LASTEXITCODE -ne 0) { throw "桌面构建失败，退出码：$LASTEXITCODE" }
    } finally {
        Pop-Location
    }
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw '构建命令完成后没有找到桌面程序。'
    }
    Set-Content -LiteralPath $stamp -Value 'Desktop build completed.' -Encoding Ascii
    Write-Host '当前桌面程序已生成，正在打开。'
    exit 0
} catch {
    Write-Host "Neothesia 无法打开：$($_.Exception.Message)" -ForegroundColor Red
    exit 1
}
