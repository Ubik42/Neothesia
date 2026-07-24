param(
    [Parameter(Mandatory = $true, ParameterSetName = "Midi")]
    [string]$MidiPath,

    [Parameter(Mandatory = $true, ParameterSetName = "CompletionFixture")]
    [switch]$CompletionFixture,

    [Parameter(Mandatory = $true, ParameterSetName = "ExerciseFixture")]
    [switch]$ExerciseFixture,

    [string]$Executable = "target\debug\neothesia.exe",

    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$repository = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
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

$midi = $null
if ($CompletionFixture) {
    $midi = Join-Path $runDirectory "completion-fixture.mid"
    # Type-1, 480 PPQ, 4/4 at 120 BPM: one C5 right-hand note and one C3
    # left-hand note at beat two, followed by enough time to finish the take.
    $fixtureBase64 = @(
        "TVRoZAAAAAYAAQADAeBNVHJrAAAAFAD/UQMHoSAA/1gEBAIYCI8A/y8ATVRyawAA"
        "AB0A/wMKUmlnaHQgSGFuZINgkEhQgXCASACJMP8vAE1UcmsAAAAcAP8DCUxlZnQg"
        "SGFuZINgkDBQgXCAMACJMP8vAA=="
    ) -join ""
    [System.IO.File]::WriteAllBytes(
        $midi,
        [System.Convert]::FromBase64String($fixtureBase64)
    )
}
elseif (-not $ExerciseFixture) {
    $midi = (Resolve-Path -LiteralPath $MidiPath).Path
}

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

function Invoke-AcceptedAction(
    [string]$Action,
    [int]$Attempts = 30
) {
    $response = $null
    for ($attempt = 0; $attempt -lt $Attempts; $attempt++) {
        $response = Invoke-DebugDriver "ACTION $Action"
        if ($response.ok -and $response.accepted) {
            return $response
        }
        Start-Sleep -Milliseconds 100
    }
    return $response
}

$process = $null
try {
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $executablePath
    $startInfo.WorkingDirectory = $runDirectory
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.Environment["NEOTHESIA_DEBUG_DRIVER_ADDR"] = "127.0.0.1:$port"
    if (-not $ExerciseFixture) {
        $startInfo.ArgumentList.Add($midi)
    }
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

    if ($ExerciseFixture) {
        # The driver can accept connections just before the first rendered frame
        # registers its clickable regions, so give the menu action a short retry
        # window instead of treating startup timing as a product failure.
        $openExercises = Invoke-AcceptedAction "practice.menu.exercises"
        Assert-True (
            $openExercises.ok -and $openExercises.accepted
        ) "Technique Studio did not open"
        $nextKey = Invoke-DebugDriver "ACTION practice.exercise.key.next"
        $previousKey = Invoke-DebugDriver "ACTION practice.exercise.key.previous"
        Assert-True (
            $nextKey.ok -and $nextKey.accepted -and
            $previousKey.ok -and $previousKey.accepted
        ) "Exercise key selector did not move in both directions"
        for ($keyStep = 0; $keyStep -lt 4; $keyStep++) {
            $selectGSharp = Invoke-DebugDriver (
                "ACTION practice.exercise.key.previous"
            )
            Assert-True (
                $selectGSharp.ok -and $selectGSharp.accepted
            ) "Exercise key selector did not reach G-sharp"
        }
        $previousTonality = Invoke-DebugDriver (
            "ACTION practice.exercise.tonality.previous"
        )
        $nextTonality = Invoke-DebugDriver (
            "ACTION practice.exercise.tonality.next"
        )
        Assert-True (
            $previousTonality.ok -and $previousTonality.accepted -and
            $nextTonality.ok -and $nextTonality.accepted
        ) "Exercise minor-form selector did not move in both directions"
        $selectNaturalMinor = Invoke-DebugDriver (
            "ACTION practice.exercise.tonality.next"
        )
        Assert-True (
            $selectNaturalMinor.ok -and $selectNaturalMinor.accepted
        ) "Exercise selector did not reach natural minor"
        $selectHarmonicMinor = Invoke-DebugDriver (
            "ACTION practice.exercise.tonality.next"
        )
        Assert-True (
            $selectHarmonicMinor.ok -and $selectHarmonicMinor.accepted
        ) "Exercise selector did not reach harmonic minor"
        $selectMelodicMinor = Invoke-DebugDriver (
            "ACTION practice.exercise.tonality.next"
        )
        Assert-True (
            $selectMelodicMinor.ok -and $selectMelodicMinor.accepted
        ) "Exercise selector did not reach melodic minor"
        $selectArpeggio = Invoke-DebugDriver (
            "ACTION practice.exercise.pattern.next"
        )
        Assert-True (
            $selectArpeggio.ok -and $selectArpeggio.accepted
        ) "Exercise selector did not reach arpeggio"
        $nextTempo = Invoke-DebugDriver "ACTION practice.exercise.tempo.next"
        Assert-True (
            $nextTempo.ok -and $nextTempo.accepted
        ) "Exercise tempo selector did not advance"
        $previousRepetitions = Invoke-DebugDriver (
            "ACTION practice.exercise.repetitions.previous"
        )
        $nextRepetitions = Invoke-DebugDriver (
            "ACTION practice.exercise.repetitions.next"
        )
        $setTwoRepetitions = Invoke-DebugDriver (
            "ACTION practice.exercise.repetitions.next"
        )
        Assert-True (
            $previousRepetitions.ok -and $previousRepetitions.accepted -and
            $nextRepetitions.ok -and $nextRepetitions.accepted -and
            $setTwoRepetitions.ok -and $setTwoRepetitions.accepted
        ) "Exercise repetition selector did not move in both directions"
        $saveFavourite = Invoke-DebugDriver (
            "ACTION practice.exercise.favourite.toggle"
        )
        $cycleFavourite = Invoke-DebugDriver (
            "ACTION practice.exercise.favourite.next"
        )
        Assert-True (
            $saveFavourite.ok -and $saveFavourite.accepted -and
            $cycleFavourite.ok -and $cycleFavourite.accepted
        ) "Exercise favourite was not saved and restorable"
        $start = Invoke-DebugDriver "ACTION practice.exercise.start"
        Assert-True (
            $start.ok -and $start.accepted
        ) "Generated exercise was not started"
    }
    else {
        $start = Invoke-DebugDriver "ACTION practice.menu.start"
        Assert-True ($start.ok -and $start.accepted) "Loaded song was not started"
    }

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
    if ($ExerciseFixture) {
        Assert-True (
            $player.fingerings_available -and $player.fingerings_enabled
        ) "Reviewed G-sharp minor-arpeggio fingering was not enabled by default"
        Assert-True (
            [int]$player.fingering_crossing_count -gt 0
        ) "Reviewed exercise did not expose any highlighted hand turns"
        $toggleFingeringsOff = Invoke-DebugDriver (
            "ACTION practice.player.fingerings"
        )
        $fingeringOff = (Invoke-DebugDriver "SNAPSHOT").snapshot
        $toggleFingeringsOn = Invoke-DebugDriver (
            "ACTION practice.player.fingerings"
        )
        Assert-True (
            $toggleFingeringsOff.ok -and $toggleFingeringsOff.accepted -and
            -not $fingeringOff.fingerings_enabled -and
            $toggleFingeringsOn.ok -and $toggleFingeringsOn.accepted
        ) "Reviewed fingering toggle did not work in both states"
    }

    if ($CompletionFixture) {
        $completion = $null
        for ($attempt = 0; $attempt -lt 200; $attempt++) {
            $candidate = (Invoke-DebugDriver "SNAPSHOT").snapshot
            if ($null -ne $candidate.completion_tab) {
                $completion = $candidate
                break
            }
            foreach ($note in @($candidate.required_note_pitches)) {
                $noteOn = Invoke-DebugDriver "MIDI 0 $note 100"
                Assert-True (
                    $noteOn.ok -and $noteOn.accepted
                ) "Completion fixture note-on was rejected"
                $noteOff = Invoke-DebugDriver "MIDI 0 $note 0"
                Assert-True (
                    $noteOff.ok -and $noteOff.accepted
                ) "Completion fixture note-off was rejected"
            }
            Start-Sleep -Milliseconds 50
        }
        Assert-True ($null -ne $completion) "Completion fixture did not finish"
        Assert-True (
            $completion.completion_tab -eq "overview"
        ) "Completion did not open on Overview"
        Assert-True ($completion.matched_notes -ge 2) "Fixture notes were not scored"

        $technique = Invoke-DebugDriver "ACTION practice.completion.tab.technique"
        Assert-True (
            $technique.ok -and $technique.accepted
        ) "Technique tab action was rejected"
        Assert-True (
            (Invoke-DebugDriver "SNAPSHOT").snapshot.completion_tab -eq "technique"
        ) "Technique tab did not become active"

        $history = Invoke-DebugDriver "ACTION practice.completion.tab.history"
        Assert-True (
            $history.ok -and $history.accepted
        ) "History tab action was rejected"
        Assert-True (
            (Invoke-DebugDriver "SNAPSHOT").snapshot.completion_tab -eq "history"
        ) "History tab did not become active"

        $overview = Invoke-DebugDriver "ACTION practice.completion.tab.overview"
        Assert-True (
            $overview.ok -and $overview.accepted
        ) "Overview tab action was rejected"
        Assert-True (
            (Invoke-DebugDriver "SNAPSHOT").snapshot.completion_tab -eq "overview"
        ) "Overview tab did not become active"

        $retry = Invoke-DebugDriver "ACTION practice.completion.retry"
        Assert-True ($retry.ok -and $retry.accepted) "Completion Retry was rejected"
        $afterRetry = (Invoke-DebugDriver "SNAPSHOT").snapshot
        Assert-True (
            $null -eq $afterRetry.completion_tab
        ) "Retry did not leave the completion screen"
        Assert-True (
            $afterRetry.matched_notes -eq 0
        ) "Retry did not reset the scored attempt"

        $back = Invoke-DebugDriver "ACTION practice.player.back"
        Assert-True ($back.ok -and $back.accepted) "Return-to-menu action was rejected"
        Start-Sleep -Milliseconds 100
        Assert-True (
            $null -eq (Invoke-DebugDriver "SNAPSHOT").snapshot
        ) "Player did not return to menu"

        $exit = Invoke-DebugDriver "EXIT"
        Assert-True $exit.ok "Clean debug exit was not acknowledged"
        if (-not $process.WaitForExit(5000)) {
            throw "Neothesia did not exit within five seconds"
        }

        return [pscustomobject]@{
            Midi = $midi
            CompletionTab = $completion.completion_tab
            MatchedNotes = [int]$completion.matched_notes
            TechniqueTab = "passed"
            HistoryTab = "passed"
            OverviewTab = "passed"
            RetryReset = [int]$afterRetry.matched_notes
            ExitCode = $process.ExitCode
        }
    }

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
    if ($ExerciseFixture) {
        Assert-True (
            @($waiting.required_note_pitches).Contains(44) -and
            @($waiting.required_note_pitches).Contains(68)
        ) "G-sharp exercise did not expose the selected two-hand tonic"
    }

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

    if ($ExerciseFixture) {
        $exerciseCompletion = $null
        for ($attempt = 0; $attempt -lt 1200; $attempt++) {
            $candidate = (Invoke-DebugDriver "SNAPSHOT").snapshot
            if ($null -ne $candidate.completion_tab) {
                $exerciseCompletion = $candidate
                break
            }
            foreach ($note in @($candidate.required_note_pitches)) {
                $noteOn = Invoke-DebugDriver "MIDI 0 $note 100"
                Assert-True (
                    $noteOn.ok -and $noteOn.accepted
                ) "Generated exercise note-on was rejected"
                $noteOff = Invoke-DebugDriver "MIDI 0 $note 0"
                Assert-True (
                    $noteOff.ok -and $noteOff.accepted
                ) "Generated exercise note-off was rejected"
            }
            Start-Sleep -Milliseconds 30
        }
        Assert-True (
            $null -ne $exerciseCompletion
        ) "Generated exercise did not reach completion"
        $retryExercise = Invoke-DebugDriver "ACTION practice.completion.retry"
        Assert-True (
            $retryExercise.ok -and $retryExercise.accepted
        ) "Generated exercise retry was rejected"
        $player = (Invoke-DebugDriver "SNAPSHOT").snapshot
    }

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

    $libraryReopen = $null
    if ($ExerciseFixture) {
        $openExercisesAgain = Invoke-AcceptedAction "practice.menu.exercises"
        $restoreRecent = Invoke-DebugDriver "ACTION practice.exercise.recent.next"
        $restoreFavourite = Invoke-DebugDriver (
            "ACTION practice.exercise.favourite.next"
        )
        $startRestored = Invoke-DebugDriver "ACTION practice.exercise.start"
        Assert-True (
            $openExercisesAgain.ok -and $openExercisesAgain.accepted -and
            $restoreRecent.ok -and $restoreRecent.accepted -and
            $restoreFavourite.ok -and $restoreFavourite.accepted -and
            $startRestored.ok -and $startRestored.accepted
        ) "Saved exercise variants were not restorable"
        for ($attempt = 0; $attempt -lt 30; $attempt++) {
            Start-Sleep -Milliseconds 100
            if ($null -ne (Invoke-DebugDriver "SNAPSHOT").snapshot) {
                break
            }
        }
        $backFromRestored = Invoke-DebugDriver "ACTION practice.player.back"
        Assert-True (
            $backFromRestored.ok -and $backFromRestored.accepted
        ) "Restored exercise did not return to the menu"
        Start-Sleep -Milliseconds 100

        $openLibrary = Invoke-DebugDriver "ACTION practice.menu.library"
        Assert-True (
            $openLibrary.ok -and $openLibrary.accepted
        ) "Practice Library did not open"
        $openRecentExercise = Invoke-DebugDriver (
            "ACTION practice.library.open-recent-exercise"
        )
        Assert-True (
            $openRecentExercise.ok -and $openRecentExercise.accepted
        ) "Generated exercise could not be reopened from Practice Library"
        for ($attempt = 0; $attempt -lt 30; $attempt++) {
            Start-Sleep -Milliseconds 100
            $candidate = (Invoke-DebugDriver "SNAPSHOT").snapshot
            if ($null -ne $candidate) {
                $libraryReopen = $candidate
                break
            }
        }
        Assert-True (
            $null -ne $libraryReopen
        ) "Reopened library exercise did not enter the player"
        Assert-True (
            $libraryReopen.hands -eq "Right"
        ) "Reopened exercise did not restore the latest hand setup"
        $backFromReopen = Invoke-DebugDriver "ACTION practice.player.back"
        Assert-True (
            $backFromReopen.ok -and $backFromReopen.accepted
        ) "Reopened exercise did not return to the menu"
        Start-Sleep -Milliseconds 100
    }

    $exit = Invoke-DebugDriver "EXIT"
    Assert-True $exit.ok "Clean debug exit was not acknowledged"
    if (-not $process.WaitForExit(5000)) {
        throw "Neothesia did not exit within five seconds"
    }

    $exercisePersistence = $null
    if ($ExerciseFixture) {
        $settingsPath = Join-Path $runDirectory "settings.ron"
        Assert-True (
            [System.IO.File]::Exists($settingsPath)
        ) "Exercise run did not persist settings"
        $settingsText = [System.IO.File]::ReadAllText($settingsPath)
        $settingsChecks = [ordered]@{
            "last exercise" = "last_exercise_spec"
            "G-sharp tonic" = "tonic:\s*8"
            "minor tonality" = "tonality:\s*Minor"
            "natural form reset" = "minor_form:\s*Natural"
            "arpeggio pattern" = "pattern:\s*Arpeggio"
            "two repetitions" = "repetitions:\s*2"
            "70 BPM" = "tempo_bpm:\s*70"
            "fingering preference" = "exercise_fingerings:\s*true"
            "recent exercises" = "recent_exercise_specs:\s*\["
            "favourite exercises" = "favourite_exercise_specs:\s*\["
        }
        foreach ($check in $settingsChecks.GetEnumerator()) {
            Assert-True (
                $settingsText -match $check.Value
            ) "Settings did not persist $($check.Key)"
        }
        $historyPath = Join-Path $runDirectory "practice-history.ron"
        Assert-True (
            [System.IO.File]::Exists($historyPath)
        ) "Completed exercise did not persist practice history"
        $historyText = [System.IO.File]::ReadAllText($historyPath)
        Assert-True (
            $historyText -match "effective_tempo_bpm:\s*Some\(70\)" -and
            $historyText -match "exercise_passes" -and
            $historyText -match "pass:\s*2"
        ) "Completed exercise did not persist its BPM and two-pass evidence"
        $exercisePersistence = (
            "G-sharp minor arpeggio 70 BPM two-pass preset and attempt saved"
        )
    }

    [pscustomobject]@{
        Source = if ($ExerciseFixture) { "generated exercise" } else { $midi }
        ExercisePersistence = $exercisePersistence
        LibraryReopen = if ($libraryReopen) { $libraryReopen.hands } else { $null }
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
