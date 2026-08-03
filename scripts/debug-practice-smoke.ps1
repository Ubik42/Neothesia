param(
    [Parameter(Mandatory = $true, ParameterSetName = "Midi")]
    [string]$MidiPath,

    [Parameter(Mandatory = $true, ParameterSetName = "CompletionFixture")]
    [switch]$CompletionFixture,

    [Parameter(Mandatory = $true, ParameterSetName = "ExerciseFixture")]
    [switch]$ExerciseFixture,

    [Parameter(Mandatory = $true, ParameterSetName = "FingeringFixture")]
    [switch]$FingeringFixture,

    [Parameter(Mandatory = $true, ParameterSetName = "ScoreFixture")]
    [switch]$ScoreFixture,

    [string]$Executable = "target\debug\neothesia.exe",

    [string]$VerovioPackageRoot = "D:\cs\_test\neothesia-verovio\node_modules\verovio",

    [string]$ScreenshotPath,

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
    $buildArguments = @(
        "build",
        "-p", "neothesia",
        "--manifest-path", (Join-Path $repository "Cargo.toml")
    )
    if ($ScoreFixture) {
        $buildArguments += @("--features", "score-verovio")
    }
    & cargo @buildArguments
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
if ($CompletionFixture -or $FingeringFixture -or $ScoreFixture) {
    $midi = Join-Path $runDirectory "completion-fixture.mid"
    # Type-1, 480 PPQ, 4/4 at 120 BPM: one C-major right-hand chord and one C3
    # left-hand note at beat two, followed by enough time to finish the take.
    $fixtureBase64 = @(
        "TVRoZAAAAAYAAQADAeBNVHJrAAAAFAD/UQMHoSAA/1gEBAIYCI8A/y8ATVRyawAA"
        "AC0A/wMKUmlnaHQgSGFuZINgkEhQAJBMUACQT1CBcIBIAACATAAAgE8AiTD/LwBN"
        "VHJrAAAAHAD/AwlMZWZ0IEhhbmSDYJAwUIFwgDAAiTD/LwA="
    ) -join ""
    [System.IO.File]::WriteAllBytes(
        $midi,
        [System.Convert]::FromBase64String($fixtureBase64)
    )
}
elseif (-not $ExerciseFixture) {
    $midi = (Resolve-Path -LiteralPath $MidiPath).Path
}

$score = $null
if ($ScoreFixture) {
    $score = Join-Path $runDirectory "score-fixture.musicxml"
    $scoreXml = @'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 3.1 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">
<score-partwise version="3.1">
  <work><work-title>Score render fixture</work-title></work>
  <part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
  <part id="P1">
    <measure number="1">
      <attributes>
        <divisions>1</divisions><key><fifths>0</fifths></key>
        <time><beats>4</beats><beat-type>4</beat-type></time>
        <clef><sign>G</sign><line>2</line></clef>
      </attributes>
      <note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><type>whole</type></note>
    </measure>
  </part>
</score-partwise>
'@
    [System.IO.File]::WriteAllText(
        $score,
        $scoreXml,
        [System.Text.UTF8Encoding]::new($false)
    )
    & cargo run -q `
        --manifest-path (Join-Path $repository "Cargo.toml") `
        -p neothesia-core `
        --example score-associate `
        -- $midi $score
    if ($LASTEXITCODE -ne 0) {
        throw "Score fixture association failed with exit code $LASTEXITCODE"
    }
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

function Invoke-SnapshotResponse([int]$Attempts = 5) {
    $lastError = $null
    for ($attempt = 0; $attempt -lt $Attempts; $attempt++) {
        try {
            $response = Invoke-DebugDriver "SNAPSHOT"
            if ($response.ok -and
                $null -ne $response.PSObject.Properties["snapshot"]) {
                return $response
            }
            $lastError = "driver returned no snapshot property"
        }
        catch {
            $lastError = $_.Exception.Message
        }
        Start-Sleep -Milliseconds 100
    }
    throw "Snapshot failed after $Attempts attempts: $lastError"
}

function Get-PracticeSnapshot {
    return (Invoke-SnapshotResponse).snapshot
}

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) {
        throw $Message
    }
}

function Save-ProcessWindowScreenshot(
    [System.Diagnostics.Process]$TargetProcess,
    [string]$Path
) {
    Add-Type -AssemblyName System.Drawing
    if (-not ("NeothesiaSmoke.NativeWindow" -as [type])) {
        Add-Type @'
using System;
using System.Runtime.InteropServices;
namespace NeothesiaSmoke {
    public static class NativeWindow {
        [StructLayout(LayoutKind.Sequential)]
        public struct RECT { public int Left, Top, Right, Bottom; }
        [DllImport("user32.dll")]
        public static extern bool GetWindowRect(IntPtr handle, out RECT rect);
        [DllImport("user32.dll")]
        public static extern bool ShowWindow(IntPtr handle, int command);
        [DllImport("user32.dll")]
        public static extern bool PrintWindow(IntPtr handle, IntPtr deviceContext, uint flags);
        [DllImport("user32.dll")]
        public static extern uint GetDpiForWindow(IntPtr handle);
    }
}
'@
    }
    $TargetProcess.Refresh()
    $handle = $TargetProcess.MainWindowHandle
    Assert-True ($handle -ne [IntPtr]::Zero) "Neothesia has no capturable window"
    [NeothesiaSmoke.NativeWindow]::ShowWindow($handle, 9) | Out-Null
    $rect = [NeothesiaSmoke.NativeWindow+RECT]::new()
    Assert-True (
        [NeothesiaSmoke.NativeWindow]::GetWindowRect($handle, [ref]$rect)
    ) "Could not read the Neothesia window bounds"
    $dpiScale = [NeothesiaSmoke.NativeWindow]::GetDpiForWindow($handle) / 96.0
    $width = [int][Math]::Round(($rect.Right - $rect.Left) * $dpiScale)
    $height = [int][Math]::Round(($rect.Bottom - $rect.Top) * $dpiScale)
    Assert-True ($width -gt 0 -and $height -gt 0) "Neothesia window bounds are empty"
    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $parent = [System.IO.Path]::GetDirectoryName($fullPath)
    if ($parent) {
        [System.IO.Directory]::CreateDirectory($parent) | Out-Null
    }
    $bitmap = [System.Drawing.Bitmap]::new($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $deviceContext = $graphics.GetHdc()
        try {
            Assert-True (
                [NeothesiaSmoke.NativeWindow]::PrintWindow($handle, $deviceContext, 2)
            ) "Windows could not render the Neothesia window for capture"
        }
        finally {
            $graphics.ReleaseHdc($deviceContext)
        }
        $bitmap.Save($fullPath, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $graphics.Dispose()
        $bitmap.Dispose()
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
    if ($ScoreFixture) {
        $packageRoot = (Resolve-Path -LiteralPath $VerovioPackageRoot).Path
        $startInfo.Environment["NEOTHESIA_VEROVIO_PACKAGE_ROOT"] = $packageRoot
        $startInfo.Environment["NEOTHESIA_VEROVIO_WORKER"] = (
            Join-Path $repository "scripts\verovio-render-worker.mjs"
        )
        $startInfo.Environment["NEOTHESIA_SCORE_CACHE"] = (
            Join-Path $runDirectory "score-cache"
        )
    }
    if (-not $ExerciseFixture) {
        # Windows PowerShell can run on a .NET version without
        # ProcessStartInfo.ArgumentList. Quote the one positional MIDI path for
        # both that runtime and modern PowerShell.
        $startInfo.Arguments = '"' + $midi.Replace('"', '\"') + '"'
    }
    $process = [System.Diagnostics.Process]::Start($startInfo)

    $menuSnapshot = $null
    for ($attempt = 0; $attempt -lt 100; $attempt++) {
        Start-Sleep -Milliseconds 100
        if ($process.HasExited) {
            throw "Neothesia exited early with code $($process.ExitCode)"
        }
        try {
            $menuSnapshot = Invoke-SnapshotResponse
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
        $selectPrimaryChords = Invoke-DebugDriver (
            "ACTION practice.exercise.pattern.next"
        )
        Assert-True (
            $selectPrimaryChords.ok -and $selectPrimaryChords.accepted
        ) "Exercise selector did not reach primary chords"
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
        $candidate = Invoke-SnapshotResponse
        if ($null -ne $candidate.snapshot) {
            $player = $candidate.snapshot
            break
        }
    }
    Assert-True ($null -ne $player) "Player scene did not become active"
    Assert-True $player.wait_for_notes "Wait-for-notes did not default to on"
    $scoreSnapshot = $null
    $scoreToggleRestored = $null
    if ($ScoreFixture) {
        for ($attempt = 0; $attempt -lt 150; $attempt++) {
            Start-Sleep -Milliseconds 100
            $candidate = Get-PracticeSnapshot
            if ($candidate.score_texture_page -eq 0) {
                $scoreSnapshot = $candidate
                break
            }
        }
        Assert-True (
            $null -ne $scoreSnapshot -and
            $scoreSnapshot.score_artifact_ready -and
            $scoreSnapshot.score_synchronization_ready -and
            [int]$scoreSnapshot.score_cached_pages -eq 1 -and
            [int]$scoreSnapshot.score_focused_page -eq 0 -and
            [int]$scoreSnapshot.score_texture_page -eq 0 -and
            [int]$scoreSnapshot.score_texture_width -gt 0 -and
            [int]$scoreSnapshot.score_texture_height -gt 0 -and
            $scoreSnapshot.score_visible
        ) "Verified score page did not reach the focused GPU texture"
        $toggleScoreOff = Invoke-DebugDriver "ACTION practice.player.score"
        $scoreOff = Get-PracticeSnapshot
        Assert-True (
            $toggleScoreOff.ok -and $toggleScoreOff.accepted -and
            -not $scoreOff.score_visible -and
            $scoreOff.score_artifact_ready -and
            [int]$scoreOff.score_cached_pages -eq 1 -and
            [int]$scoreOff.score_focused_page -eq 0 -and
            $null -eq $scoreOff.score_texture_page
        ) "Score toggle did not hide and release only the GPU texture"
        $toggleScoreOn = Invoke-DebugDriver "ACTION practice.player.score"
        $scoreOn = Get-PracticeSnapshot
        Assert-True (
            $toggleScoreOn.ok -and $toggleScoreOn.accepted -and
            $scoreOn.score_visible -and
            [int]$scoreOn.score_texture_page -eq 0 -and
            [int]$scoreOn.score_texture_width -eq [int]$scoreSnapshot.score_texture_width -and
            [int]$scoreOn.score_texture_height -eq [int]$scoreSnapshot.score_texture_height
        ) "Score toggle did not restore the cached focused texture"
        $scoreToggleRestored = "hidden with cache retained, then restored"
        if ($ScreenshotPath) {
            Start-Sleep -Milliseconds 250
            Save-ProcessWindowScreenshot $process $ScreenshotPath
        }
    }
    if ($ExerciseFixture) {
        Assert-True (
            $player.fingerings_available -and $player.fingerings_enabled
        ) "Reviewed G-sharp minor primary-chord fingering was not enabled by default"
        Assert-True (
            [int]$player.fingering_crossing_count -eq 0
        ) "Block-chord fingering incorrectly exposed hand-turn highlights"
        $toggleFingeringsOff = Invoke-DebugDriver (
            "ACTION practice.player.fingerings"
        )
        $fingeringOff = Get-PracticeSnapshot
        $toggleFingeringsOn = Invoke-DebugDriver (
            "ACTION practice.player.fingerings"
        )
        Assert-True (
            $toggleFingeringsOff.ok -and $toggleFingeringsOff.accepted -and
            -not $fingeringOff.fingerings_enabled -and
            $toggleFingeringsOn.ok -and $toggleFingeringsOn.accepted
        ) "Reviewed fingering toggle did not work in both states"
    }

    if ($FingeringFixture) {
        Assert-True (
            -not $player.fingerings_available -and
            [int]$player.manual_fingering_count -eq 0
        ) "Fresh fingering fixture unexpectedly contained hints"
        $openFingeringEditor = Invoke-DebugDriver (
            "ACTION practice.player.fingering-editor"
        )
        $editing = Get-PracticeSnapshot
        Assert-True (
            $openFingeringEditor.ok -and $openFingeringEditor.accepted -and
            $editing.fingering_editor_active -and $editing.paused
        ) "Finger editor did not open on a paused imported MIDI"

        $selectChord = Invoke-DebugDriver (
            "ACTION practice.player.fingering-next"
        )
        Assert-True (
            $selectChord.ok -and $selectChord.accepted
        ) "Finger editor did not select the right-hand chord"

        $suggest = Invoke-DebugDriver (
            "ACTION practice.player.fingering-suggest"
        )
        $suggested = Get-PracticeSnapshot
        Assert-True (
            $suggest.ok -and $suggest.accepted -and
            [int]$suggested.suggested_finger -eq 1 -and
            [int]$suggested.suggested_fingering_count -eq 3 -and
            [int]$suggested.suggestion_confidence_percent -eq 78
        ) "Right-hand C-major chord did not preview all three fingers at 78% confidence"

        $assign = Invoke-DebugDriver (
            "ACTION practice.player.fingering-accept"
        )
        $assigned = Get-PracticeSnapshot
        Assert-True (
            $assign.ok -and $assign.accepted -and
            $assigned.fingering_editor_active -and
            $assigned.fingerings_available -and
            $assigned.fingerings_enabled -and
            [int]$assigned.fingering_count -eq 3 -and
            [int]$assigned.manual_fingering_count -eq 3
        ) "Accepting the chord suggestion did not update all three live hints"

        $closeFingeringEditor = Invoke-DebugDriver (
            "ACTION practice.player.fingering-editor"
        )
        Assert-True (
            $closeFingeringEditor.ok -and $closeFingeringEditor.accepted
        ) "Finger editor did not close"
        $back = Invoke-DebugDriver "ACTION practice.player.back"
        Assert-True ($back.ok -and $back.accepted) "Return-to-menu action was rejected"
        Start-Sleep -Milliseconds 100
        $exit = Invoke-DebugDriver "EXIT"
        Assert-True $exit.ok "Clean debug exit was not acknowledged"
        if (-not $process.WaitForExit(5000)) {
            throw "Neothesia did not exit within five seconds"
        }

        $sidecarPath = "$midi.neothesia.ron"
        Assert-True (
            [System.IO.File]::Exists($sidecarPath)
        ) "Finger edit did not create an adjacent sidecar"
        $sidecarText = [System.IO.File]::ReadAllText($sidecarPath)
        Assert-True (
            $sidecarText -match "fingerings:\s*\[" -and
            ([regex]::Matches($sidecarText, "track_id:")).Count -eq 3 -and
            ([regex]::Matches($sidecarText, "note_index:")).Count -eq 3 -and
            ([regex]::Matches($sidecarText, "finger:\s*[1-5]")).Count -eq 3
        ) "Saved sidecar did not contain all three accepted chord hints"

        return [pscustomobject]@{
            Midi = $midi
            EditorPaused = [bool]$editing.paused
            SuggestedFinger = [int]$suggested.suggested_finger
            SuggestedCount = [int]$suggested.suggested_fingering_count
            SuggestionConfidence = [int]$suggested.suggestion_confidence_percent
            FingeringCount = [int]$assigned.fingering_count
            ManualFingeringCount = [int]$assigned.manual_fingering_count
            Sidecar = $sidecarPath
            ExitCode = $process.ExitCode
        }
    }

    if ($CompletionFixture) {
        $completion = $null
        for ($attempt = 0; $attempt -lt 200; $attempt++) {
            $candidate = Get-PracticeSnapshot
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
            (Get-PracticeSnapshot).completion_tab -eq "technique"
        ) "Technique tab did not become active"

        $history = Invoke-DebugDriver "ACTION practice.completion.tab.history"
        Assert-True (
            $history.ok -and $history.accepted
        ) "History tab action was rejected"
        Assert-True (
            (Get-PracticeSnapshot).completion_tab -eq "history"
        ) "History tab did not become active"

        $overview = Invoke-DebugDriver "ACTION practice.completion.tab.overview"
        Assert-True (
            $overview.ok -and $overview.accepted
        ) "Overview tab action was rejected"
        Assert-True (
            (Get-PracticeSnapshot).completion_tab -eq "overview"
        ) "Overview tab did not become active"

        $retry = Invoke-DebugDriver "ACTION practice.completion.retry"
        Assert-True ($retry.ok -and $retry.accepted) "Completion Retry was rejected"
        $afterRetry = Get-PracticeSnapshot
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
            $null -eq (Get-PracticeSnapshot)
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
        $candidate = Get-PracticeSnapshot
        if ($candidate.required_notes -gt 0) {
            $waiting = $candidate
            break
        }
        Start-Sleep -Milliseconds 100
    }
    Assert-True ($null -ne $waiting) "Player did not expose a required note"
    $requiredPitches = @(
        $waiting.required_note_pitches | ForEach-Object { [int]$_ }
    )
    Assert-True (
        $requiredPitches.Count -eq $waiting.required_notes
    ) "Required-note count and pitch list disagree"
    if ($ExerciseFixture) {
        Assert-True (
            $requiredPitches.Contains(44) -and
            $requiredPitches.Contains(68)
        ) (
            "G-sharp exercise did not expose the selected two-hand tonic; " +
            "required pitches were: " +
            ($requiredPitches -join ", ")
        )
    }

    $matchedBeforeInput = [int]$waiting.matched_notes
    foreach ($note in $requiredPitches) {
        $noteOn = Invoke-DebugDriver "MIDI 0 $note 100"
        Assert-True ($noteOn.ok -and $noteOn.accepted) "Debug note-on was rejected"
        $noteOff = Invoke-DebugDriver "MIDI 0 $note 0"
        Assert-True ($noteOff.ok -and $noteOff.accepted) "Debug note-off was rejected"
    }
    $afterInput = Get-PracticeSnapshot
    Assert-True (
        $afterInput.matched_notes -gt $matchedBeforeInput
    ) "Injected performance notes did not reach the practice matcher"

    if ($ExerciseFixture) {
        $exerciseCompletion = $null
        for ($attempt = 0; $attempt -lt 1200; $attempt++) {
            $candidate = Get-PracticeSnapshot
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
        $player = Get-PracticeSnapshot
    }

    $waitBefore = [bool]$player.wait_for_notes
    $toggle = Invoke-DebugDriver "ACTION practice.player.wait"
    Assert-True ($toggle.ok -and $toggle.accepted) "Wait toggle was rejected"
    $afterToggle = Get-PracticeSnapshot
    Assert-True (
        [bool]$afterToggle.wait_for_notes -ne $waitBefore
    ) "Wait state did not change"

    $afterHands = $null
    if ($null -ne $player.hands) {
        $hands = Invoke-DebugDriver "ACTION practice.player.hands"
        Assert-True ($hands.ok -and $hands.accepted) "Hand mode cycle was rejected"
        $afterHands = Get-PracticeSnapshot
        Assert-True ($afterHands.hands -ne $player.hands) "Hand mode did not change"
    }

    $loop = Invoke-DebugDriver "ACTION practice.player.loop"
    Assert-True ($loop.ok -and $loop.accepted) "Loop toggle was rejected"
    $loopState = Get-PracticeSnapshot
    Assert-True $loopState.loop_active "Loop did not become active"
    Assert-True (
        $null -ne $loopState.loop_start_measure -and
        $null -ne $loopState.loop_end_measure -and
        $loopState.loop_start_measure -le $loopState.loop_end_measure
    ) "Loop did not expose a valid measure range"

    $restart = Invoke-DebugDriver "ACTION practice.player.restart"
    Assert-True ($restart.ok -and $restart.accepted) "Practice restart was rejected"
    $afterRestart = Get-PracticeSnapshot
    Assert-True $afterRestart.loop_active "Restart unexpectedly disabled the loop"
    Assert-True (
        $afterRestart.loop_start_measure -eq $loopState.loop_start_measure -and
        $afterRestart.loop_end_measure -eq $loopState.loop_end_measure
    ) "Restart did not preserve the loop range"

    $loopOff = Invoke-DebugDriver "ACTION practice.player.loop"
    Assert-True ($loopOff.ok -and $loopOff.accepted) "Loop disable was rejected"
    $afterLoopOff = Get-PracticeSnapshot
    Assert-True (-not $afterLoopOff.loop_active) "Loop remained active after toggling off"

    $back = Invoke-DebugDriver "ACTION practice.player.back"
    Assert-True ($back.ok -and $back.accepted) "Return-to-menu action was rejected"
    Start-Sleep -Milliseconds 100
    $menuAgain = Invoke-SnapshotResponse
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
            if ($null -ne (Get-PracticeSnapshot)) {
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
            $candidate = Get-PracticeSnapshot
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
    if ($ScoreFixture) {
        $settingsPath = Join-Path $runDirectory "settings.ron"
        Assert-True (
            [System.IO.File]::Exists($settingsPath)
        ) "Score run did not persist settings"
        $settingsText = [System.IO.File]::ReadAllText($settingsPath)
        Assert-True (
            $settingsText -match "score_visible:\s*true"
        ) "Settings did not persist the restored score visibility"
    }
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
            "primary-chord pattern" = "pattern:\s*PrimaryChords"
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
            "G-sharp minor primary chords 70 BPM two-pass preset and attempt saved"
        )
    }

    [pscustomobject]@{
        Source = if ($ExerciseFixture) { "generated exercise" } else { $midi }
        ExercisePersistence = $exercisePersistence
        LibraryReopen = if ($libraryReopen) { $libraryReopen.hands } else { $null }
        ScoreTexture = if ($scoreSnapshot) {
            "$($scoreSnapshot.score_texture_width)x$($scoreSnapshot.score_texture_height)"
        } else { $null }
        ScoreToggle = $scoreToggleRestored
        Screenshot = if ($ScreenshotPath) {
            [System.IO.Path]::GetFullPath($ScreenshotPath)
        } else { $null }
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
