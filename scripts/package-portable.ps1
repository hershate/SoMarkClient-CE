param(
  [string]$Version = '0.1.3'
)

$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..')
$executable = Join-Path $root 'src-tauri\target\release\somarkclient-ce.exe'
$staging = Join-Path $root 'TEMP\portable-package'
$release = Join-Path $root 'release'

if (-not (Test-Path $executable)) {
  throw "Release executable not found: $executable"
}

if (Test-Path $staging) {
  Remove-Item -Recurse -Force $staging
}
New-Item -ItemType Directory -Force -Path $staging, $release | Out-Null
Copy-Item $executable (Join-Path $staging 'SoMarkClient CE.exe')
Copy-Item (Join-Path $root 'LICENSE') (Join-Path $staging 'LICENSE.txt')
Set-Content -NoNewline -Path (Join-Path $staging 'portable.flag') -Value 'portable'
$archive = Join-Path $release "SoMarkClient-CE-$Version-portable.zip"
if (Test-Path $archive) {
  Remove-Item -Force $archive
}
Compress-Archive -Path (Join-Path $staging '*') -DestinationPath $archive -CompressionLevel Optimal

$nsisDirectory = Join-Path $root 'src-tauri\target\release\bundle\nsis'
if (Test-Path $nsisDirectory) {
  Get-ChildItem -Path $nsisDirectory -Filter '*.exe' | ForEach-Object {
    Copy-Item $_.FullName (Join-Path $release $_.Name) -Force
  }
}

Remove-Item -Recurse -Force $staging
Write-Output $archive
