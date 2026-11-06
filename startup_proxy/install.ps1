# Installe (ou retire avec -Uninstall) le proxy de démarrage, winmm.dll, à côté de l'exe.
#   powershell -ExecutionPolicy Bypass -File startup_proxy\install.ps1
#   powershell -ExecutionPolicy Bypass -File startup_proxy\install.ps1 -Uninstall
# Aucun fichier du jeu n'est remplacé : une vérification des fichiers ou une mise à jour par Steam ne
# retire pas le proxy. C'est lui qui charge le proxy de GameCore (proxy\install.ps1).
# Une installation d'avant la version 0.4 (proxy à la place de DatabaseDLL_FinalRelease.dll, original
# sous DatabaseDLL_FinalRelease_orig.dll) est d'abord remise en place, comme le fait l'installeur.
param(
    [switch]$Uninstall,
    [string]$GameDir = "C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI"
)
$ErrorActionPreference = "Stop"

$bin = Join-Path $GameDir "Base\Binaries\Win64Steam"
$proxy = Join-Path $bin "winmm.dll"
$old = Join-Path $bin "DatabaseDLL_FinalRelease.dll"
$oldOrig = Join-Path $bin "DatabaseDLL_FinalRelease_orig.dll"
if (-not (Test-Path $old)) { throw "DatabaseDLL_FinalRelease.dll introuvable dans $bin : ce n'est pas le dossier du jeu." }
if (Get-Process | Where-Object { $_.Name -like "CivilizationVI*" }) { throw "Ferme Civilization VI d'abord." }

function Test-Ours($path) {
    if (-not (Test-Path $path)) { return $false }
    [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($path)).Contains("StartupProxyStatus")
}

# Ancienne installation : l'original revient ; un _orig à côté d'un original remis par Steam est périmé.
if (Test-Ours $old) {
    if (-not (Test-Path $oldOrig)) { throw "Un ancien proxy est à la place de l'original mais $oldOrig manque : vérifie les fichiers du jeu dans Steam." }
    Remove-Item $old
    Rename-Item $oldOrig (Split-Path $old -Leaf)
    Write-Host "Ancienne installation retirée : DatabaseDLL_FinalRelease.dll est de nouveau l'original."
} elseif (Test-Path $oldOrig) {
    Remove-Item $oldOrig
    Write-Host "$oldOrig périmé retiré."
}

if ($Uninstall) {
    if (Test-Ours $proxy) { Remove-Item $proxy }
    Write-Host "Proxy de démarrage retiré."
    return
}

if ((Test-Path $proxy) -and -not (Test-Ours $proxy)) { throw "$proxy n'est pas le proxy de démarrage (un autre mod ?) : retire-le d'abord." }
$built = Join-Path $PSScriptRoot "target\release\winmm.dll"
if (-not (Test-Path $built)) { throw "Compile d'abord : cargo build --release dans $PSScriptRoot" }
Copy-Item $built $proxy -Force
Write-Host "Proxy de démarrage installé : $proxy."
Write-Host "Journal : $env:LOCALAPPDATA\Civ6RolloverProxy\RolloverProxy.log"
