$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $Root
$Package = Get-Content (Join-Path $Root 'package.json') -Raw | ConvertFrom-Json
$Version = $Package.version

Write-Host '=== GitFlic Contour Sync Windows Release ===' -ForegroundColor Cyan
Write-Host "Root: $Root"

foreach ($cmd in @('node','npm','cargo','rustc')) {
  if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
    throw "Не найден $cmd. Сборку выполняйте на машине разработки, где установлены Node.js/npm и Rust."
  }
}

node --version
npm --version
rustc --version
cargo --version

Write-Host '\n[1/3] Production build...' -ForegroundColor Cyan
npm run tauri:build
if ($LASTEXITCODE -ne 0) { throw "tauri:build завершился с кодом $LASTEXITCODE" }

$TargetRelease = Join-Path $Root 'src-tauri\target\release'
$Artifacts = Join-Path $Root 'artifacts\windows'
if (Test-Path $Artifacts) { Remove-Item $Artifacts -Recurse -Force }
New-Item -ItemType Directory -Path $Artifacts -Force | Out-Null

Write-Host '\n[2/3] Collecting artifacts...' -ForegroundColor Cyan
$Portable = Join-Path $TargetRelease 'gitflic-contour-sync.exe'
if (-not (Test-Path $Portable)) { throw "Не найден release EXE: $Portable" }
Copy-Item $Portable (Join-Path $Artifacts 'GitFlic-Contour-Sync-portable.exe')

$Bundles = Get-ChildItem (Join-Path $TargetRelease 'bundle') -Recurse -File -ErrorAction SilentlyContinue |
  Where-Object { $_.Extension -in '.exe','.msi' }
foreach ($file in $Bundles) { Copy-Item $file.FullName (Join-Path $Artifacts $file.Name) }

@'
@echo off
start "" "%~dp0GitFlic-Contour-Sync-portable.exe"
'@ | Set-Content (Join-Path $Artifacts 'Start-GitFlic-Contour-Sync.bat') -Encoding ASCII

$HashFile = Join-Path $Artifacts 'SHA256SUMS.txt'
Get-ChildItem $Artifacts -File | Where-Object { $_.Extension -in '.exe','.msi' } | ForEach-Object {
  $h = Get-FileHash $_.FullName -Algorithm SHA256
  "$($h.Hash)  $($_.Name)"
} | Set-Content $HashFile -Encoding ASCII

$Zip = Join-Path $Root ("artifacts\GitFlic-Contour-Sync-v$Version-windows.zip")
if (Test-Path $Zip) { Remove-Item $Zip -Force }
Compress-Archive -Path (Join-Path $Artifacts '*') -DestinationPath $Zip -CompressionLevel Optimal

Write-Host '\n[3/3] Result' -ForegroundColor Green
Get-ChildItem $Artifacts -File | Select-Object Name,Length,FullName | Format-Table -AutoSize
Write-Host "ZIP: $Zip" -ForegroundColor Green
Write-Host 'Copy the portable EXE or installer to the closed PROD contour. npm/Rust are not required there.' -ForegroundColor Green
