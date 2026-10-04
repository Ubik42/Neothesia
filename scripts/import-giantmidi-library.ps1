param(
    [string]$LibraryRoot = "D:\Music\MIDI\PracticeLibrary",
    [string]$RepositoryRoot = "D:\Music\_tools\_reference\GiantMIDI-Piano",
    # Omit this to import the four repository previews. Supply an extracted
    # official release directory to import the full or surname-checked release.
    [string]$MidiRoot = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$LibraryRoot = [System.IO.Path]::GetFullPath($LibraryRoot)
$RepositoryRoot = [System.IO.Path]::GetFullPath($RepositoryRoot)
$isPreview = [string]::IsNullOrWhiteSpace($MidiRoot)
if ($isPreview) { $MidiRoot = Join-Path $RepositoryRoot "midis_preview" }
$MidiRoot = [System.IO.Path]::GetFullPath($MidiRoot)
if (-not [System.IO.Directory]::Exists($MidiRoot)) {
    throw "MIDI source directory not found: $MidiRoot. Clone GiantMIDI-Piano or supply -MidiRoot."
}
$collection = "Classical_Transcription_GiantMIDI"
$destination = Join-Path $LibraryRoot $collection
if ($MidiRoot.Equals($LibraryRoot, [StringComparison]::OrdinalIgnoreCase) -or
    $MidiRoot.StartsWith($LibraryRoot.TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Use a source directory outside LibraryRoot to avoid importing library copies."
}

# The upstream .csv is tab-separated. Match the complete basename, since a
# YouTube performance ID can appear against more than one work in the metadata.
$metadata = [System.Collections.Generic.Dictionary[string, object]]::new([StringComparer]::Ordinal)
$metadataPath = Join-Path $RepositoryRoot "resources/full_music_pieces_youtube_similarity_pianosoloprob.csv"
if ([System.IO.File]::Exists($metadataPath)) {
    Import-Csv -LiteralPath $metadataPath -Delimiter "`t" | ForEach-Object {
        if (-not [string]::IsNullOrWhiteSpace($_.audio_name)) { $metadata[$_.audio_name] = $_ }
    }
}
$source = "GiantMIDI-Piano"
$sourceUrl = "https://github.com/bytedance/GiantMIDI-Piano"
$catalogPath = Join-Path $LibraryRoot "catalog.csv"
$rows = [System.Collections.Generic.List[object]]::new()
$existingPaths = @{}
if ([System.IO.File]::Exists($catalogPath)) {
    foreach ($row in Import-Csv -LiteralPath $catalogPath) {
        $rows.Add($row)
        $existingPaths[$row.LocalPath.Replace('/', '\')] = $row
    }
}
$files = @(Get-ChildItem -LiteralPath $MidiRoot -Recurse -File |
    Where-Object { $_.Extension -in @('.mid', '.midi') } | Sort-Object FullName)
if ($files.Count -eq 0) { throw "No MIDI files found in $MidiRoot" }
$plans = [System.Collections.Generic.List[object]]::new()
$originalNames = @{}
$nameMapPath = Join-Path $MidiRoot 'filename-map.csv'
if ([System.IO.File]::Exists($nameMapPath)) {
    foreach ($mapping in Import-Csv -LiteralPath $nameMapPath) {
        $originalNames[$mapping.LocalPath.Replace('/', '\')] = $mapping.OriginalBaseName
    }
}
$skipped = 0
foreach ($file in $files) {
    $stream = [System.IO.File]::OpenRead($file.FullName)
    try {
        $header = [byte[]]::new(14)
        $read = $stream.Read($header, 0, $header.Length)
    } finally { $stream.Dispose() }
    if ($read -ne 14 -or [System.Text.Encoding]::ASCII.GetString($header, 0, 4) -ne 'MThd' -or
        $header[4] -ne 0 -or $header[5] -ne 0 -or $header[6] -ne 0 -or $header[7] -ne 6 -or
        $header[8] -ne 0 -or $header[9] -gt 2 -or ($header[10] -eq 0 -and $header[11] -eq 0)) {
        Write-Warning "Skipping invalid MIDI header: $($file.FullName)"
        $skipped++
        continue
    }
    $relative = [System.IO.Path]::GetRelativePath($MidiRoot, $file.FullName)
    $localPath = Join-Path $collection $relative
    $target = Join-Path $LibraryRoot $localPath
    $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
    if ([System.IO.File]::Exists($target) -and
        (Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash -ne $hash) {
        throw "Destination has different contents; refusing to overwrite: $target"
    }
    if ($existingPaths.ContainsKey($localPath) -and $existingPaths[$localPath].Source -ne $source) {
        throw "Catalogue path belongs to another source: $localPath"
    }
    $originalName = if ($originalNames.ContainsKey($relative.Replace('/', '\'))) {
        $originalNames[$relative.Replace('/', '\')]
    } else { $file.BaseName }
    $entry = if ($metadata.ContainsKey($originalName)) { $metadata[$originalName] } else { $null }
    if ($null -ne $entry) {
        $title = $entry.music
        $composer = "$($entry.firstname) $($entry.surname)".Trim()
    } else {
        # Standard release names: surname, firstname, work (with commas), video ID.
        $parts = $originalName -split ', '
        $title = $originalName
        $composer = "Unknown"
        if ($parts.Count -ge 4) {
            $composer = "$($parts[1]) $($parts[0])"
            $title = $parts[2..($parts.Count - 2)] -join ', '
        }
    }
    $row = [pscustomobject]@{
        Category = "Classical transcription/GiantMIDI (unreviewed)"
        Title = $title
        Composer = $composer
        License = "CC BY 4.0; see upstream disclaimer"
        Source = $source
        SourceUrl = $sourceUrl
        LocalPath = $localPath
    }
    $plans.Add([pscustomobject]@{ File = $file.FullName; Target = $target; Row = $row; SHA256 = $hash })
}
if ($plans.Count -eq 0) { throw "No valid MIDI files to import; catalogue unchanged." }

# Preflight every collision before copying anything. Repeated imports preserve
# unrelated sources and already imported GiantMIDI performances.
[System.IO.Directory]::CreateDirectory($destination) | Out-Null
$importedPaths = @{}
foreach ($plan in $plans) {
    [System.IO.Directory]::CreateDirectory([System.IO.Path]::GetDirectoryName($plan.Target)) | Out-Null
    if (-not [System.IO.File]::Exists($plan.Target)) {
        [System.IO.File]::Copy($plan.File, $plan.Target, $false)
    }
    $importedPaths[$plan.Row.LocalPath] = $true
}
$merged = @($rows | Where-Object { -not $importedPaths.ContainsKey($_.LocalPath.Replace('/', '\')) }) +
    @($plans | ForEach-Object Row)
$temporaryCatalog = "$catalogPath.$([Guid]::NewGuid().ToString('N')).tmp"
try {
    $merged | Sort-Object Category, Composer, Title, LocalPath |
        Export-Csv -LiteralPath $temporaryCatalog -NoTypeInformation -Encoding utf8BOM
    [System.IO.File]::Move($temporaryCatalog, $catalogPath, $true)
} finally {
    if ([System.IO.File]::Exists($temporaryCatalog)) { Remove-Item -LiteralPath $temporaryCatalog }
}
$revision = $null
if ([System.IO.Directory]::Exists((Join-Path $RepositoryRoot '.git'))) {
    $revision = & git -C $RepositoryRoot rev-parse HEAD
    if ($LASTEXITCODE -ne 0) { $revision = $null }
}
[pscustomobject]@{
    Source = $source
    Repository = "https://github.com/Ubik42/GiantMIDI-Piano"
    Revision = $revision
    Upstream = $sourceUrl
    ImportedAt = [DateTimeOffset]::UtcNow.ToString('o')
    Selection = $(if ($isPreview) { 'repository previews' } else { 'supplied release directory' })
    ReviewStatus = 'Automatic transcription; not reviewed teaching scores or hand assignments'
    Files = @($plans | ForEach-Object { [pscustomobject]@{ LocalPath = $_.Row.LocalPath; SHA256 = $_.SHA256 } })
} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $destination 'last-import.json') -Encoding utf8
[pscustomobject]@{
    LibraryRoot = $LibraryRoot
    ImportedMidi = $plans.Count
    SkippedInvalidMidi = $skipped
    CatalogRows = $merged.Count
    Catalog = $catalogPath
}
