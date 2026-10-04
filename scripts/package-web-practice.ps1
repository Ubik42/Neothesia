param([Parameter(Mandatory)][string]$Destination)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repository = Split-Path $PSScriptRoot -Parent
Push-Location (Join-Path $repository 'neothesia-web')
try { & npm.cmd run desktop:build:dev; if ($LASTEXITCODE) { throw '桌面构建失败' } } finally { Pop-Location }
$Destination = [System.IO.Path]::GetFullPath($Destination)
[System.IO.Directory]::CreateDirectory($Destination) | Out-Null
Copy-Item -LiteralPath (Join-Path $repository 'target/debug/neothesia-desktop.exe') -Destination (Join-Path $Destination 'Neothesia.exe')
Copy-Item -LiteralPath (Join-Path $repository 'default.sf2') -Destination (Join-Path $Destination 'default.sf2')
Copy-Item -LiteralPath (Join-Path $repository 'LICENSE') -Destination (Join-Path $Destination 'LICENSE')
$launcherLines = @('@echo off', 'set NEOTHESIA_HEADLESS_CHECK=', 'start "" /D "%~dp0" "%~dp0Neothesia.exe" %*')
[System.IO.File]::WriteAllText((Join-Path $Destination 'Open.cmd'), (($launcherLines -join "`r`n") + "`r`n"), [System.Text.Encoding]::ASCII)
Copy-Item -LiteralPath (Join-Path $repository 'docs/development/desktop-guide.txt') -Destination (Join-Path $Destination '使用说明.txt')
Write-Output "运行包已生成：$Destination"

$notices=Join-Path $Destination 'ThirdParty/Verovio'
[System.IO.Directory]::CreateDirectory($notices)|Out-Null
Copy-Item -LiteralPath (Join-Path $repository 'neothesia-web/public/verovio/NOTICE.md'),(Join-Path $repository 'neothesia-web/public/verovio/COPYING'),(Join-Path $repository 'neothesia-web/public/verovio/COPYING.LESSER') -Destination $notices

$pdfNotice=Join-Path $Destination 'ThirdParty/PDF.js'
[System.IO.Directory]::CreateDirectory($pdfNotice)|Out-Null
Copy-Item -LiteralPath (Join-Path $repository 'neothesia-web/public/pdfjs/LICENSE') -Destination $pdfNotice -Force
