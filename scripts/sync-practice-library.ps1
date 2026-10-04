param(
    [string]$LibraryRoot = "D:\Music\MIDI\PracticeLibrary",
    [int]$PopKCount = 256,
    [switch]$IncludeGiantMidi,
    [string]$GiantMidiRoot = "",
    [string]$GiantMidiRepository = "D:\Music\_tools\_reference\GiantMIDI-Piano",
    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$archiveRoot = Join-Path $LibraryRoot "_archives"
$maestroRoot = Join-Path $LibraryRoot "Classical_Performance_MAESTRO"
$mutopiaRoot = Join-Path $LibraryRoot "Classical_Teaching_Mutopia"
$popRoot = Join-Path $LibraryRoot "Open_Pop_Style"
@($LibraryRoot, $archiveRoot, $maestroRoot, $mutopiaRoot, $popRoot) |
    ForEach-Object { [System.IO.Directory]::CreateDirectory($_) | Out-Null }

function Get-HexHash {
    param([string]$Path, [string]$Algorithm)
    (Get-FileHash -LiteralPath $Path -Algorithm $Algorithm).Hash.ToLowerInvariant()
}

function Get-VerifiedArchive {
    param(
        [string]$Uri,
        [string]$Path,
        [string]$Algorithm,
        [string]$ExpectedHash
    )
    if ($Force -or -not [System.IO.File]::Exists($Path)) {
        Invoke-WebRequest -Uri $Uri -OutFile $Path -UseBasicParsing
    }
    $actual = Get-HexHash $Path $Algorithm
    if ($actual -ne $ExpectedHash.ToLowerInvariant()) {
        throw "Archive checksum mismatch: $Path ($actual)"
    }
}

function Convert-HtmlText {
    param([string]$Value)
    $withoutTags = [regex]::Replace($Value, "<[^>]+>", " ")
    $decoded = [System.Net.WebUtility]::HtmlDecode($withoutTags)
    ([regex]::Replace($decoded, "\s+", " ")).Trim()
}

function Get-MutopiaRows {
    param(
        [string]$Query,
        [string]$Category,
        [string]$Destination,
        [switch]$PianoOnly
    )
    [System.IO.Directory]::CreateDirectory($Destination) | Out-Null
    $rows = [System.Collections.Generic.List[object]]::new()
    for ($start = 0; $start -lt 2000; $start += 10) {
        $separator = if ($Query.Contains("?")) { "&" } else { "?" }
        $uri = "$Query${separator}startat=$start"
        $response = Invoke-WebRequest -Uri $uri -UseBasicParsing
        $html = if ($response.Content -is [byte[]]) {
            [System.Text.Encoding]::UTF8.GetString($response.Content)
        } else {
            [string]$response.Content
        }
        $blocks = [regex]::Matches(
            $html,
            '<table class="table-bordered result-table">(.*?)</table>',
            [System.Text.RegularExpressions.RegexOptions]::Singleline
        )
        if ($blocks.Count -eq 0) { break }
        foreach ($blockMatch in $blocks) {
            $block = $blockMatch.Groups[1].Value
            $instrument = if ($block -match '<td>for\s+([^<]+)</td>') {
                Convert-HtmlText $Matches[1]
            } else { "" }
            if ($PianoOnly -and $instrument -notmatch '(?i)Piano') { continue }
            if ($block -notmatch '<tr><td>(.*?)</td>\s*<td>(.*?)</td>') { continue }
            $title = Convert-HtmlText $Matches[1]
            $composer = (Convert-HtmlText $Matches[2]) -replace '^by\s+', ''
            $license = if ($block -match '<a href="[^\"]*legal\.html[^\"]*">(.*?)</a>') {
                Convert-HtmlText $Matches[1]
            } else { "See source page" }
            $midiMatch = [regex]::Match(
                $block,
                'href="([^"]+\.(?:mid|midi|zip))"[^>]*>[^<]*\.mid',
                [System.Text.RegularExpressions.RegexOptions]::IgnoreCase
            )
            if (-not $midiMatch.Success) { continue }
            $sourceUrl = [System.Net.WebUtility]::HtmlDecode($midiMatch.Groups[1].Value)
            $remoteName = [System.IO.Path]::GetFileName(([Uri]$sourceUrl).AbsolutePath)
            $target = Join-Path $Destination $remoteName
            $needsDownload = $Force -or -not [System.IO.File]::Exists($target) -or
                ([System.IO.File]::Exists($target) -and (Get-Item -LiteralPath $target).Length -eq 0)
            if ($needsDownload) {
                try {
                    Invoke-WebRequest -Uri $sourceUrl -OutFile $target -UseBasicParsing
                } catch {
                    if ([System.IO.File]::Exists($target) -and
                        (Get-Item -LiteralPath $target).Length -eq 0) {
                        Remove-Item -LiteralPath $target -Force
                    }
                    Write-Warning "Skipping unavailable Mutopia MIDI: $sourceUrl ($($_.Exception.Message))"
                    continue
                }
            }
            if ([System.IO.Path]::GetExtension($target) -ieq ".zip") {
                $packageRoot = Join-Path $Destination ([System.IO.Path]::GetFileNameWithoutExtension($target))
                if ($Force -or -not [System.IO.Directory]::Exists($packageRoot)) {
                    Expand-Archive -LiteralPath $target -DestinationPath $packageRoot -Force
                }
                foreach ($midi in Get-ChildItem -LiteralPath $packageRoot -Recurse -File -Include *.mid, *.midi) {
                    $rows.Add([pscustomobject]@{
                        Category = $Category
                        Title = $title
                        Composer = $composer
                        License = $license
                        Source = "Mutopia Project"
                        SourceUrl = $sourceUrl
                        LocalPath = [System.IO.Path]::GetRelativePath($LibraryRoot, $midi.FullName)
                    })
                }
            } else {
                $rows.Add([pscustomobject]@{
                    Category = $Category
                    Title = $title
                    Composer = $composer
                    License = $license
                    Source = "Mutopia Project"
                    SourceUrl = $sourceUrl
                    LocalPath = [System.IO.Path]::GetRelativePath($LibraryRoot, $target)
                })
            }
        }
    }
    $rows
}

$maestroArchive = Join-Path $archiveRoot "maestro-v3.0.0-midi.zip"
Get-VerifiedArchive `
    -Uri "https://storage.googleapis.com/magentadata/datasets/maestro/v3.0.0/maestro-v3.0.0-midi.zip" `
    -Path $maestroArchive `
    -Algorithm "SHA256" `
    -ExpectedHash "70470ee253295c8d2c71e6d9d4a815189e35c89624b76d22fce5a019d5dde12c"
if ($Force -or (Get-ChildItem -LiteralPath $maestroRoot -Recurse -File -Include *.mid, *.midi).Count -eq 0) {
    Expand-Archive -LiteralPath $maestroArchive -DestinationPath $maestroRoot -Force
}

$popArchive = Join-Path $archiveRoot "popk_dataset_300k_mid.tar.gz"
Get-VerifiedArchive `
    -Uri "https://zenodo.org/records/14791511/files/popk_dataset_300k_mid.tar.gz?download=1" `
    -Path $popArchive `
    -Algorithm "MD5" `
    -ExpectedHash "25decb80e7c1e4395694e90aa6f39f76"
$popNames = 1..$PopKCount | ForEach-Object { "popk_dataset_{0:D6}.mid" -f $_ }
$missingPopNames = @($popNames | Where-Object {
    -not [System.IO.File]::Exists((Join-Path $popRoot $_))
})
if ($missingPopNames.Count -gt 0) {
    & tar -xzf $popArchive -C $popRoot $missingPopNames
    if ($LASTEXITCODE -ne 0) { throw "Pop-K extraction failed with exit code $LASTEXITCODE" }
}

$mutopiaQueries = @(
    @{ Folder = "01_Burgmuller_Op100"; Category = "Teaching/Burgmuller Op.100"; Query = "https://www.mutopiaproject.org/cgibin/make-table.cgi?Composer=BurgmullerJFF" },
    @{ Folder = "02_Czerny_Technique"; Category = "Teaching/Czerny technique"; Query = "https://www.mutopiaproject.org/cgibin/make-table.cgi?Style=Technique" },
    @{ Folder = "03_Clementi"; Category = "Teaching/Clementi"; Query = "https://www.mutopiaproject.org/cgibin/make-table.cgi?Composer=ClementiM" },
    @{ Folder = "04_Bach"; Category = "Classical/Bach"; Query = "https://www.mutopiaproject.org/cgibin/make-table.cgi?Composer=BachJS" },
    @{ Folder = "05_Schumann"; Category = "Teaching/Schumann"; Query = "https://www.mutopiaproject.org/cgibin/make-table.cgi?Composer=SchumannR" },
    @{ Folder = "06_Mozart"; Category = "Classical/Mozart"; Query = "https://www.mutopiaproject.org/cgibin/make-table.cgi?Composer=MozartWA" },
    @{ Folder = "07_Chopin"; Category = "Classical/Chopin"; Query = "https://www.mutopiaproject.org/cgibin/make-table.cgi?Composer=ChopinFF" },
    @{ Folder = "08_Satie"; Category = "Classical/Satie"; Query = "https://www.mutopiaproject.org/cgibin/make-table.cgi?Composer=SatieE" },
    @{ Folder = "09_Joplin_Ragtime"; Category = "Popular style/Joplin ragtime"; Query = "https://www.mutopiaproject.org/cgibin/make-table.cgi?Composer=JoplinS" },
    @{ Folder = "10_Public_Domain_Dance"; Category = "Popular style/Historic dance"; Query = "https://www.mutopiaproject.org/cgibin/make-table.cgi?Style=Popular+%2F+Dance" }
)
$catalog = [System.Collections.Generic.List[object]]::new()
foreach ($item in $mutopiaQueries) {
    $destination = Join-Path $mutopiaRoot $item.Folder
    foreach ($row in Get-MutopiaRows -Query $item.Query -Category $item.Category -Destination $destination -PianoOnly) {
        $catalog.Add($row)
    }
}

$maestroMetadata = Get-ChildItem -LiteralPath $maestroRoot -Recurse -File -Filter "maestro-v3.0.0.csv" |
    Select-Object -First 1
if ($maestroMetadata) {
    foreach ($entry in Import-Csv -LiteralPath $maestroMetadata.FullName) {
        $midiPath = Join-Path $maestroMetadata.DirectoryName $entry.midi_filename
        if ([System.IO.File]::Exists($midiPath)) {
            $catalog.Add([pscustomobject]@{
                Category = "Classical performance/MAESTRO"
                Title = $entry.canonical_title
                Composer = $entry.canonical_composer
                License = "CC BY-NC-SA 4.0"
                Source = "MAESTRO v3.0.0"
                SourceUrl = "https://magenta.withgoogle.com/datasets/maestro"
                LocalPath = [System.IO.Path]::GetRelativePath($LibraryRoot, $midiPath)
            })
        }
    }
}

foreach ($midi in Get-ChildItem -LiteralPath $popRoot -File -Filter "popk_dataset_*.mid" | Sort-Object Name) {
    $catalog.Add([pscustomobject]@{
        Category = "Modern pop melody excerpt/Pop-K"
        Title = $midi.BaseName
        Composer = "Patchbanks dataset"
        License = "CC BY-NC 4.0"
        Source = "Pop-K v1.0 curated subset"
        SourceUrl = "https://doi.org/10.5281/zenodo.14791511"
        LocalPath = [System.IO.Path]::GetRelativePath($LibraryRoot, $midi.FullName)
    })
}

$catalogPath = Join-Path $LibraryRoot "catalog.csv"
# Preserve collections imported independently, including GiantMIDI. This sync
# owns only these three source families, not every row in the local library.
if ([System.IO.File]::Exists($catalogPath)) {
    foreach ($row in Import-Csv -LiteralPath $catalogPath) {
        if ($row.Source -notin @("Mutopia Project", "MAESTRO v3.0.0", "Pop-K v1.0 curated subset")) {
            $catalog.Add($row)
        }
    }
}
$catalogRows = @($catalog | Sort-Object LocalPath -Unique)
$temporaryCatalog = "$catalogPath.$([Guid]::NewGuid().ToString('N')).tmp"
try {
    $catalogRows |
        Sort-Object Category, Composer, Title, LocalPath |
        Export-Csv -LiteralPath $temporaryCatalog -NoTypeInformation -Encoding utf8BOM
    [System.IO.File]::Move($temporaryCatalog, $catalogPath, $true)
} finally {
    if ([System.IO.File]::Exists($temporaryCatalog)) { Remove-Item -LiteralPath $temporaryCatalog }
}

if ($IncludeGiantMidi) {
    & (Join-Path $PSScriptRoot "import-giantmidi-library.ps1") `
        -LibraryRoot $LibraryRoot -RepositoryRoot $GiantMidiRepository -MidiRoot $GiantMidiRoot
}

[pscustomobject]@{
    LibraryRoot = $LibraryRoot
    Catalog = $catalogPath
    MaestroMidi = (Get-ChildItem -LiteralPath $maestroRoot -Recurse -File -Include *.mid, *.midi).Count
    MutopiaMidi = (Get-ChildItem -LiteralPath $mutopiaRoot -Recurse -File -Include *.mid, *.midi).Count
    PopKMidi = (Get-ChildItem -LiteralPath $popRoot -File -Filter *.mid).Count
    CatalogRows = @(Import-Csv -LiteralPath $catalogPath).Count
}
