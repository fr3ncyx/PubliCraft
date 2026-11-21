# Pubblica su GitHub Releases la versione corrente di PubliCraft, firmata con la tua chiave privata.
# Servono GitHub CLI (`gh auth login`) e la chiave creata con `francysign keygen`.
# Uso:  .\publish-release.ps1 "Note della versione"
param([string]$Notes = "Aggiornamento di PubliCraft")
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$sign = Join-Path $PSScriptRoot '..\FrancyLauncher\target\release\francysign.exe'
if (-not (Test-Path $sign)) { throw "francysign non trovato: compila FrancyLauncher (cargo build --release)" }
cargo build --release -p publicraft
$exe = Join-Path $PSScriptRoot 'target\release\publicraft.exe'
& $sign sign $exe
if ($LASTEXITCODE -ne 0) { throw "firma non riuscita" }
$ver = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
gh release create "v$ver" $exe "$exe.sig" --repo fr3ncyx/PubliCraft --title "PubliCraft $ver" --notes $Notes
Write-Host "Pubblicata la v$ver (exe + firma)"
