# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Copyright (c) 2026 2youg1 and the sprawling contributors
#
# One command that leaves `sprawling` on your PATH:
#
#     irm https://raw.githubusercontent.com/2youg1/sprawling/main/install.ps1 | iex
#
# This script fetches and unpacks. **Where the binary goes, and what
# happens to PATH, is decided by `sprawling install`** - the binary's own
# installer, which runs at the end. Two installers choosing a directory
# would be two authorities for one rule, and the one a person can re-run
# later is the one that has to win.
#
# Three facts about this repository's releases that the obvious script
# gets wrong:
#
#   * Every release so far is a pre-release, and GitHub's `releases/latest`
#     endpoint excludes those - it answers 404 here. The list endpoint is
#     asked for its newest entry instead.
#   * The tag and the archive carry different versions: tag
#     `v0.0.3-Pre-alpha-260903` ships `sprawling-0.0.3-windows-x86_64.zip`.
#     An asset is chosen by its platform suffix, never by a name built out
#     of the tag.
#   * Only the platforms the release workflow builds are installable. A
#     platform with no archive is reported with the list the release does
#     carry, rather than guessed at.
#
# `sprawling` is the short name GitHub still resolves to this repository,
# whose canonical path is `2youg1/sprawling-agents`. Set SPRAWLING_REPO if
# that ever stops being true.

$ErrorActionPreference = 'Stop'

$repo = if ($env:SPRAWLING_REPO) { $env:SPRAWLING_REPO } else { '2youg1/sprawling' }
$api = "https://api.github.com/repos/$repo/releases"
$releases = "https://github.com/$repo/releases"

function Die($message) {
    Write-Error "sprawling install: $message"
    exit 1
}

function Format-Megabytes($bytes) {
    '{0:N1}' -f ($bytes / 1MB)
}

# One line for the whole download: what is being fetched, for which
# platform, how far it has come, and how far that is in percent.
function Format-DownloadLine($label, $done, $total) {
    if ($total -gt 0) {
        $percent = [int][Math]::Floor($done * 100 / $total)
        "sprawling $label | $(Format-Megabytes $done) / $(Format-Megabytes $total) MB | $percent%"
    } else {
        "sprawling $label | $(Format-Megabytes $done) MB"
    }
}

# A console lets the line rewrite itself in place. A file or a pipe does
# not: a carriage return per redraw leaves a log nobody can read, so
# there the download prints its closing line and nothing else.
$redraws = -not [Console]::IsOutputRedirected

function Write-Redraw($line) {
    if ($redraws) { Write-Host ("`r" + $line.PadRight(78)) -NoNewline }
}

function Write-Closing($line) {
    if ($redraws) { Write-Host ("`r" + $line.PadRight(78)) } else { Write-Host $line }
}

# Stream the archive to disk and draw that line as it arrives.
#
# `Invoke-WebRequest`'s own progress bar is not used: Windows PowerShell
# redraws it per chunk, and on a several-megabyte file that redraw is
# most of the wall clock. Ten milliseconds between redraws looks
# continuous to a reader and costs nothing on a fast link.
function Save-Download($url, $path, $label, $total) {
    if (-not ('System.Net.Http.HttpClient' -as [type])) {
        Add-Type -AssemblyName System.Net.Http
    }
    $client = [Net.Http.HttpClient]::new()
    $client.Timeout = [TimeSpan]::FromMinutes(30)
    $client.DefaultRequestHeaders.Add('User-Agent', 'sprawling-install')
    try {
        $headersRead = [Net.Http.HttpCompletionOption]::ResponseHeadersRead
        $response = $client.GetAsync($url, $headersRead).GetAwaiter().GetResult()
        if (-not $response.IsSuccessStatusCode) {
            Die ("$url answered $([int]$response.StatusCode) $($response.ReasonPhrase); " +
                 "download from $releases instead")
        }
        if ($response.Content.Headers.ContentLength) {
            $total = $response.Content.Headers.ContentLength
        }
        $source = $response.Content.ReadAsStreamAsync().GetAwaiter().GetResult()
        $sink = [IO.FileStream]::new(
            $path, [IO.FileMode]::Create, [IO.FileAccess]::Write, [IO.FileShare]::None)
        try {
            $buffer = New-Object byte[] 81920
            $done = [long]0
            $clock = [Diagnostics.Stopwatch]::StartNew()
            $drawn = [long]-1000
            Write-Redraw (Format-DownloadLine $label $done $total)
            while ($true) {
                $read = $source.Read($buffer, 0, $buffer.Length)
                if ($read -le 0) { break }
                $sink.Write($buffer, 0, $read)
                $done += $read
                if (($clock.ElapsedMilliseconds - $drawn) -ge 10) {
                    Write-Redraw (Format-DownloadLine $label $done $total)
                    $drawn = $clock.ElapsedMilliseconds
                }
            }
            Write-Closing (Format-DownloadLine $label $done $total)
        } finally {
            $sink.Dispose()
            $source.Dispose()
        }
    } finally {
        $client.Dispose()
    }
}

# Windows PowerShell 5.1 still negotiates SSL3/TLS1.0 by default on some
# builds, and GitHub answers those with a closed connection rather than
# with an error a reader can act on. PowerShell 7 already defaults to the
# system's choice, so this is set rather than forced.
if ([Net.ServicePointManager]::SecurityProtocol -notmatch 'Tls12') {
    [Net.ServicePointManager]::SecurityProtocol =
        [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
}

$arch = switch ($env:PROCESSOR_ARCHITECTURE) {
    'AMD64' { 'x86_64' }
    'ARM64' { 'aarch64' }
    'x86'   { Die 'no 32-bit archive is built; build from source with `just dist`' }
    default { Die "no archive is built for $($env:PROCESSOR_ARCHITECTURE)" }
}
$suffix = "-windows-$arch.zip"

Write-Host "sprawling: asking $repo what it has for windows-$arch"
try {
    $release = if ($env:SPRAWLING_VERSION) {
        Invoke-RestMethod -Uri "$api/tags/$($env:SPRAWLING_VERSION)" -UseBasicParsing
    } else {
        # `-1` is what the list endpoint answers with when asked for one
        # entry; PowerShell unwraps a single-element array, so the value
        # is the release either way.
        @(Invoke-RestMethod -Uri "${api}?per_page=1" -UseBasicParsing)[0]
    }
} catch {
    Die "cannot reach $api ($($_.Exception.Message)); download from $releases instead"
}
if (-not $release) { Die "no release found; see $releases" }

$asset = $release.assets | Where-Object { $_.name.EndsWith($suffix) } | Select-Object -First 1
if (-not $asset) {
    Write-Host "sprawling: $($release.tag_name) carries no archive ending $suffix. What it does carry:"
    $release.assets | ForEach-Object { Write-Host "  $($_.name)" }
    Die "download one of those from $releases, or build from source with ``just dist``"
}

# The digest the release publishes, in the form `sha256:<hex>`. Absent
# means the bytes cannot be checked, and an unverified download is not
# installed here.
if (-not $asset.digest -or -not $asset.digest.StartsWith('sha256:')) {
    Die ("$($release.tag_name) publishes no sha256 for $($asset.name), so the bytes " +
         "cannot be checked. Download it yourself from $releases if you accept that.")
}
$expected = $asset.digest.Substring(7)

$work = Join-Path ([IO.Path]::GetTempPath()) "sprawling-install-$([Guid]::NewGuid())"
New-Item -ItemType Directory -Path $work -Force | Out-Null
try {
    $archive = Join-Path $work 'archive.zip'
    Save-Download $asset.browser_download_url $archive `
        "$($release.tag_name) | windows-$arch" $asset.size

    $got = (Get-FileHash -Path $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($got -ne $expected) {
        Die ("the archive does not match the sha256 the release publishes.`n" +
             "  expected $expected`n  received $got`nNothing was installed.")
    }
    Write-Host "sprawling: verified sha256 $expected"

    Expand-Archive -Path $archive -DestinationPath $work -Force
    $binary = Get-ChildItem -Path $work -Filter 'sprawling.exe' -Recurse -File |
        Select-Object -First 1
    if (-not $binary) { Die 'the archive holds no file named sprawling.exe' }

    # What was installed, said by the thing that was installed: the first
    # line of `status` is the binary's own version. It is a courtesy and
    # not a gate, so a binary that cannot answer is passed over here and
    # reports itself to the installer below, which does gate on it.
    $reported = ''
    try { $reported = & $binary.FullName status 2>$null | Select-Object -First 1 }
    catch { $reported = '' }
    if ($reported) { Write-Host "sprawling: $reported" }

    # The binary places itself. Everything this script knows about
    # directories and PATH ends here, and what it prints below comes from
    # the installer a person can run again by hand.
    & $binary.FullName install
    if ($LASTEXITCODE -ne 0) { Die "sprawling install exited with $LASTEXITCODE" }
} finally {
    Remove-Item -Path $work -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host 'Next: run `sprawling up` to raise a city and open it in your browser.'
Write-Host 'It needs a model to call before it can do anything - `sprawling help` lists the rest.'
