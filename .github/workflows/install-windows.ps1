# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Copyright (c) 2026 2youg1 and the sprawling contributors

param([Parameter(Mandatory)][string]$Archives, [Parameter(Mandatory)][string]$Evidence)
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_OS -ne 'Windows') {
    throw 'This installer check writes the disposable Windows runner profile only.'
}
$assets = @(Get-ChildItem -LiteralPath $Archives -Filter '*.zip' -File)
if ($assets.Count -ne 1) { throw 'Expected exactly one Windows archive.' }
$root = Join-Path $env:RUNNER_TEMP "install-check-$([Guid]::NewGuid())"
New-Item -ItemType Directory -Path $root, $Evidence -Force | Out-Null
$server = $null
$before = @{ LocalAppData = $env:LOCALAPPDATA; Api = $env:SPRAWLING_API; Version = $env:SPRAWLING_VERSION }
$key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
$hadPath = $key.GetValueNames() -contains 'Path'
$pathValue = if ($hadPath) { $key.GetValue('Path', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) } else { $null }
$pathKind = if ($hadPath) { $key.GetValueKind('Path') } else { $null }
try {
    $www = Join-Path $root 'www'
    $unpacked = Join-Path $root 'unpacked'
    New-Item -ItemType Directory -Path $www -Force | Out-Null
    Copy-Item -LiteralPath $assets[0].FullName -Destination $www
    $log = Join-Path $root 'server.out'
    $err = Join-Path $root 'server.err'
    $server = Start-Process python -ArgumentList @('-u', '-m', 'http.server', '0', '--bind', '127.0.0.1', '--directory', $www) -PassThru -RedirectStandardOutput $log -RedirectStandardError $err
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    $port = $null
    while ([DateTime]::UtcNow -lt $deadline) {
        if (Test-Path -LiteralPath $log) {
            $serverOutput = Get-Content -LiteralPath $log -Raw
            if ($serverOutput) {
                $match = [regex]::Match($serverOutput, 'Serving HTTP on 127\.0\.0\.1 port ([0-9]+)')
                if ($match.Success) { $port = $match.Groups[1].Value; break }
            }
        }
        if ($server.WaitForExit(100)) { throw 'The loopback asset server exited before readiness.' }
    }
    if (-not $port) { throw 'The loopback asset server did not report its port.' }
    $hash = (Get-FileHash -LiteralPath $assets[0].FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    $release = @(@{ tag_name = 'loopback'; assets = @(@{ name = $assets[0].Name; size = $assets[0].Length; digest = "sha256:$hash"; browser_download_url = "http://127.0.0.1:$port/$($assets[0].Name)" }) })
    $release | ConvertTo-Json -Depth 5 -AsArray | Set-Content -LiteralPath (Join-Path $www 'releases.json') -Encoding utf8
    $env:LOCALAPPDATA = Join-Path $root 'profile'
    $env:SPRAWLING_API = "http://127.0.0.1:$port/releases.json"
    $env:SPRAWLING_VERSION = $null
    $installer = (Resolve-Path install.ps1).Path
    & pwsh -NoProfile -File $installer *> (Join-Path $Evidence 'powershell-install.log')
    if ($LASTEXITCODE -ne 0) { throw 'The PowerShell installer refused this build archive.' }
    Expand-Archive -LiteralPath $assets[0].FullName -DestinationPath $unpacked
    $original = @(Get-ChildItem -LiteralPath $unpacked -Recurse -Filter sprawling.exe -File)
    $installed = @(Get-ChildItem -LiteralPath $env:LOCALAPPDATA -Recurse -Filter sprawling.exe -File)
    if ($original.Count -ne 1 -or $installed.Count -ne 1) { throw 'The archive and installed profile must each hold one binary.' }
    $originalHash = (Get-FileHash -LiteralPath $original[0].FullName -Algorithm SHA256).Hash
    $installedHash = (Get-FileHash -LiteralPath $installed[0].FullName -Algorithm SHA256).Hash
    if ($originalHash -cne $installedHash) { throw 'PowerShell installed different executable bytes.' }
    $metadata = $installed[0].VersionInfo
    $signature = Get-AuthenticodeSignature -LiteralPath $installed[0].FullName
    @{ archive = $assets[0].Name; archiveSha256 = $hash; binarySha256 = $installedHash.ToLowerInvariant();
       productName = $metadata.ProductName; productVersion = $metadata.ProductVersion; fileVersion = $metadata.FileVersion;
       signatureStatus = $signature.Status.ToString(); hasTimestamp = $null -ne $signature.TimeStamperCertificate;
       run = $env:GITHUB_RUN_ID; attempt = $env:GITHUB_RUN_ATTEMPT } |
        ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $Evidence 'windows-binary.json') -Encoding utf8
    $status = & $installed[0].FullName status
    if ($LASTEXITCODE -ne 0) { throw 'The installed executable cannot answer status.' }
    & pwsh -NoProfile -File $installer *> (Join-Path $Evidence 'powershell-reinstall.log')
    if ($LASTEXITCODE -ne 0) { throw 'Same-channel reinstall refused the archive.' }
    if ((Get-FileHash -LiteralPath $installed[0].FullName -Algorithm SHA256).Hash -cne $originalHash) {
        throw 'Same-channel reinstall changed the expected executable bytes.'
    }
    $installedDir = $installed[0].DirectoryName
    $pathBeforeUninstall = $key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    $pathKindBeforeUninstall = $key.GetValueKind('Path')
    $kept = @($pathBeforeUninstall.Split(';') | Where-Object {
        -not $_.Trim().TrimEnd([char[]]'\/').Equals($installedDir.TrimEnd([char[]]'\/'), [StringComparison]::OrdinalIgnoreCase)
    })
    if ($kept.Count -eq $pathBeforeUninstall.Split(';').Count) { throw 'Installation never added its PATH entry.' }
    & $installed[0].FullName install --uninstall *> (Join-Path $Evidence 'powershell-uninstall.log')
    if ($LASTEXITCODE -ne 0 -or (Test-Path -LiteralPath $installed[0].FullName)) { throw 'Uninstall failed to remove its executable.' }
    $pathAfterUninstall = $key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    if ($pathAfterUninstall -cne ($kept -join ';') -or $key.GetValueKind('Path') -ne $pathKindBeforeUninstall) {
        throw 'Uninstall must remove only its PATH entry and preserve the value type.'
    }
    $originDefinition = [regex]::Match((Get-Content -LiteralPath 'crates/sprawling/src/release.rs' -Raw), 'pub const ARCHIVE_ORIGIN_FILE: &str = "([^"]+)";')
    if (-not $originDefinition.Success) { throw 'Archive origin authority was not found.' }
    $origin = Join-Path $installedDir $originDefinition.Groups[1].Value
    if (Test-Path -LiteralPath $origin) { throw 'Uninstall left the archive origin marker.' }

    $release[0].assets[0].digest = 'sha256:' + ('0' * 64)
    $release | ConvertTo-Json -Depth 5 -AsArray | Set-Content -LiteralPath (Join-Path $www 'releases.json') -Encoding utf8
    & pwsh -NoProfile -File $installer *> (Join-Path $Evidence 'powershell-bad-digest.log')
    if ($LASTEXITCODE -eq 0 -or (Test-Path -LiteralPath $installed[0].FullName)) { throw 'A mismatched archive digest must refuse installation.' }
    Copy-Item -LiteralPath $err -Destination (Join-Path $Evidence 'loopback-requests.log')
    if ((Get-Content -LiteralPath $err -Raw) -notmatch '/releases\.json') { throw 'The installer never requested the loopback release list.' }
    @{ archive = $assets[0].Name; archiveSha256 = $hash; binarySha256 = $installedHash.ToLowerInvariant(); status = $status; result = 'success'; isolatedTarget = 'disposable hosted runner profile'; run = $env:GITHUB_RUN_ID; attempt = $env:GITHUB_RUN_ATTEMPT } |
        ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $Evidence 'powershell-install.json') -Encoding utf8
} finally {
    if ($hadPath) { $key.SetValue('Path', $pathValue, $pathKind) } else { $key.DeleteValue('Path', $false) }
    $key.Dispose()
    $env:LOCALAPPDATA = $before.LocalAppData
    $env:SPRAWLING_API = $before.Api
    $env:SPRAWLING_VERSION = $before.Version
    if ($server -and -not $server.HasExited) { Stop-Process -Id $server.Id; $server.WaitForExit() }
    Remove-Item -LiteralPath $root -Recurse -Force
}
exit 0
