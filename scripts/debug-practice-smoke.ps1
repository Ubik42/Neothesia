param(
    [Parameter(Mandatory = $true)]
    [string]$MidiPath,

    [string]$Executable = "target\debug\neothesia.exe",

    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$repository = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$midi = (Resolve-Path -LiteralPath $MidiPath).Path
$executablePath = if ([System.IO.Path]::IsPathRooted($Executable)) {
    $Executable
}
else {
    Join-Path $repository $Executable
}

if (-not $SkipBuild) {
    & cargo build -p neothesia --manifest-path (Join-Path $repository "Cargo.toml")
    if ($LASTEXITCODE -ne 0) {
        throw "Debug build failed with exit code $LASTEXITCODE"
    }
}

$executablePath = (Resolve-Path -LiteralPath $executablePath).Path
$portProbe = [System.Net.Sockets.TcpListener]::new(
    [System.Net.IPAddress]::Loopback,
    0
)
$portProbe.Start()
$port = ([System.Net.IPEndPoint]$portProbe.LocalEndpoint).Port
$portProbe.Stop()

$runDirectory = Join-Path ([System.IO.Path]::GetTempPath()) (
    "neothesia-smoke-" + [System.Guid]::NewGuid().ToString("N")
)
[System.IO.Directory]::CreateDirectory($runDirectory) | Out-Null
Copy-Item -LiteralPath (Join-Path $repository "default.sf2") -Destination $runDirectory

function Invoke-DebugDriver([string]$Command) {
    $client = [System.Net.Sockets.TcpClient]::new()
    try {
        $client.Connect("127.0.0.1", $port)
        $stream = $client.GetStream()
        $writer = [System.IO.StreamWriter]::new(
            $stream,
            [System.Text.UTF8Encoding]::new($false),
            1024,
            $true
        )
        $reader = [System.IO.StreamReader]::new(
            $stream,
            [System.Text.Encoding]::UTF8,
            $false,
            1024,
            $true
        )
        $writer.WriteLine($Command)
        $writer.Flush()
        $line = $reader.ReadLine()
        if (-not $line) {
            throw "Debug driver returned an empty response for '$Command'"
        }
        return $line | ConvertFrom-Json
    }
    finally {
        $client.Dispose()
    }
}

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) {
        throw $Message
    }
}

$process = $null
try {
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $executablePath
    $startInfo.WorkingDirectory = $runDirectory
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.Environment["NEOTHESIA_DEBUG_DRIVER_ADDR"] = "127.0.0.1:$port"
    $startInfo.ArgumentList.Add($midi)
    $process = [System.Diagnostics.Process]::Start($startInfo)

    $menuSnapshot = $null
    for ($attempt = 0; $attempt -lt 100; $attempt++) {
        Start-Sleep -Milliseconds 100
        if ($process.HasExited) {
            throw "Neothesia exited early with code $($process.ExitCode)"
        }
        try {
            $menuSnapshot = Invoke-DebugDriver "SNAPSHOT"
            break
        }
        catch {
            if ($attempt -eq 99) {
                throw
            }
        }
    }

    Assert-True $menuSnapshot.ok "Menu snapshot request failed"
    Assert-True ($null -eq $menuSnapshot.snapshot) "Expected a menu scene snapshot"

    $start = Invoke-DebugDriver "ACTION practice.menu.start"
    Assert-True ($start.ok -and $start.accepted) "Loaded song was not started"

    $player = $null
    for ($attempt = 0; $attempt -lt 30; $attempt++) {
        Start-Sleep -Milliseconds 100
        $candidate = Invoke-DebugDriver "SNAPSHOT"
        if ($null -ne $candidate.snapshot) {
            $player = $candidate.snapshot
            break
        }
    }
    Assert-True ($null -ne $player) "Player scene did not become active"
    Assert-True $player.wait_for_notes "Wait-for-notes did not default to on"

    $waiting = $null
    for ($attempt = 0; $attempt -lt 150; $attempt++) {
        $candidate = (Invoke-DebugDriver "SNAPSHOT").snapshot
        if ($candidate.required_notes -gt 0) {
            $waiting = $candidate
            break
        }
        Start-Sleep -Milliseconds 100
    }
    Assert-True ($null -ne $waiting) "Player did not expose a required note"
    Assert-True (
        @($waiting.required_note_pitches).Count -eq $waiting.required_notes
    ) "Required-note count and pitch list disagree"

    $matchedBeforeInput = [int]$waiting.matched_notes
    foreach ($note in @($waiting.required_note_pitches)) {
        $noteOn = Invoke-DebugDriver "MIDI 0 $note 100"
        Assert-True ($noteOn.ok -and $noteOn.accepted) "Debug note-on was rejected"
        $noteOff = Invoke-DebugDriver "MIDI 0 $note 0"
        Assert-True ($noteOff.ok -and $noteOff.accepted) "Debug note-off was rejected"
    }
    $afterInput = (Invoke-DebugDriver "SNAPSHOT").snapshot
    Assert-True (
        $afterInput.matched_notes -gt $matchedBeforeInput
    ) "Injected performance notes did not reach the practice matcher"

    $waitBefore = [bool]$player.wait_for_notes
    $toggle = Invoke-DebugDriver "ACTION practice.player.wait"
    Assert-True ($toggle.ok -and $toggle.accepted) "Wait toggle was rejected"
    $afterToggle = (Invoke-DebugDriver "SNAPSHOT").snapshot
    Assert-True (
        [bool]$afterToggle.wait_for_notes -ne $waitBefore
    ) "Wait state did not change"

    $afterHands = $null
    if ($null -ne $player.hands) {
        $hands = Invoke-DebugDriver "ACTION practice.player.hands"
        Assert-True ($hands.ok -and $hands.accepted) "Hand mode cycle was rejected"
        $afterHands = (Invoke-DebugDriver "SNAPSHOT").snapshot
        Assert-True ($afterHands.hands -ne $player.hands) "Hand mode did not change"
    }

    $loop = Invoke-DebugDriver "ACTION practice.player.loop"
    Assert-True ($loop.ok -and $loop.accepted) "Loop toggle was rejected"
    $loopState = (Invoke-DebugDriver "SNAPSHOT").snapshot
    Assert-True $loopState.loop_active "Loop did not become active"
    Assert-True (
        $null -ne $loopState.loop_start_measure -and
        $null -ne $loopState.loop_end_measure -and
        $loopState.loop_start_measure -le $loopState.loop_end_measure
    ) "Loop did not expose a valid measure range"

    $restart = Invoke-DebugDriver "ACTION practice.player.restart"
    Assert-True ($restart.ok -and $restart.accepted) "Practice restart was rejected"
    $afterRestart = (Invoke-DebugDriver "SNAPSHOT").snapshot
    Assert-True $afterRestart.loop_active "Restart unexpectedly disabled the loop"
    Assert-True (
        $afterRestart.loop_start_measure -eq $loopState.loop_start_measure -and
        $afterRestart.loop_end_measure -eq $loopState.loop_end_measure
    ) "Restart did not preserve the loop range"

    $loopOff = Invoke-DebugDriver "ACTION practice.player.loop"
    Assert-True ($loopOff.ok -and $loopOff.accepted) "Loop disable was rejected"
    $afterLoopOff = (Invoke-DebugDriver "SNAPSHOT").snapshot
    Assert-True (-not $afterLoopOff.loop_active) "Loop remained active after toggling off"

    $back = Invoke-DebugDriver "ACTION practice.player.back"
    Assert-True ($back.ok -and $back.accepted) "Return-to-menu action was rejected"
    Start-Sleep -Milliseconds 100
    $menuAgain = Invoke-DebugDriver "SNAPSHOT"
    Assert-True ($null -eq $menuAgain.snapshot) "Player did not return to menu"

    $exit = Invoke-DebugDriver "EXIT"
    Assert-True $exit.ok "Clean debug exit was not acknowledged"
    if (-not $process.WaitForExit(5000)) {
        throw "Neothesia did not exit within five seconds"
    }

    [pscustomobject]@{
        Midi = $midi
        WaitDefault = $waitBefore
        WaitAfterToggle = [bool]$afterToggle.wait_for_notes
        MatchedAfterInput = [int]$afterInput.matched_notes
        HandsBefore = $player.hands
        HandsAfter = if ($afterHands) { $afterHands.hands } else { $null }
        LoopRange = "$($loopState.loop_start_measure)-$($loopState.loop_end_measure)"
        LoopRestarted = [bool]$afterRestart.loop_active
        LoopDisabled = -not [bool]$afterLoopOff.loop_active
        ExitCode = $process.ExitCode
    }
}
finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id
        $process.WaitForExit()
    }
    $resolvedRunDirectory = [System.IO.Path]::GetFullPath($runDirectory)
    $tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
    if ($resolvedRunDirectory.StartsWith($tempRoot) -and
        (Split-Path $resolvedRunDirectory -Leaf).StartsWith("neothesia-smoke-")) {
        Remove-Item -LiteralPath $resolvedRunDirectory -Recurse -Force
    }
}
