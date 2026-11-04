# Test du salon multijoueur : lance le hook Frida, puis archive ses journaux et ceux du jeu.
# Autonome : à envoyer avec hook_lobby.js et hook_lobby.py (même dossier), voir FICHE.md.
#
#   powershell -ExecutionPolicy Bypass -File test_salon.ps1 -Nom <ton pseudo>
#
# 1. Lance Civ6 en DX12 (menu principal), puis ce script : il attend le jeu et s'y attache.
# 2. Pendant le test : Entrée = photo des emplacements (à faire quand on te le demande).
# 3. À la fin : tape fin puis Entrée, ferme le jeu SANS le relancer, puis Entrée à nouveau.
#    L'archive part sur le Bureau : salon-<nom>-<date>.zip, à renvoyer.
param(
    [Parameter(Mandatory = $true)][string]$Nom,
    # Vérifie seulement l'installation (Python, Frida), sans attendre le jeu.
    [switch]$Verifier,
    # Interpréteur Python à utiliser s'il n'est pas trouvé tout seul (python.exe complet).
    [string]$Python = ""
)
# Pas "Stop" : Windows PowerShell 5.1 transforme en erreur fatale toute ligne écrite par Python sur
# stderr. Les échecs sont testés explicitement.
$ErrorActionPreference = "Continue"
$ici = Split-Path -Parent $MyInvocation.MyCommand.Path
# Dans le kit envoyé, le hook est à côté du script ; dans le dépôt, un dossier au-dessus.
if (-not (Test-Path (Join-Path $ici "hook_lobby.py"))) { $ici = Split-Path -Parent $ici }
foreach ($f in "hook_lobby.js", "hook_lobby.py") {
    if (-not (Test-Path (Join-Path $ici $f))) { throw "Fichier manquant à côté du script : $f" }
}

# --- Python et Frida, dans un environnement à part (rien d'installé ailleurs) -------------------
$FRIDA = "17.25.2"
$env_dir = Join-Path $env:LOCALAPPDATA "Civ6LobbyTest\venv"
$py = Join-Path $env_dir "Scripts\python.exe"
if (-not (Test-Path $py)) {
    $base = $null
    $cands = @()
    if ($Python) { $cands += , @($Python) }
    $cands += , @("py", "-3")
    $cands += , @("python")
    foreach ($g in @("$env:LOCALAPPDATA\Programs\Python\Python3*\python.exe", "$env:USERPROFILE\Anaconda3\python.exe",
                     "$env:USERPROFILE\miniconda3\python.exe", "$env:ProgramFiles\Python3*\python.exe")) {
        Get-ChildItem $g -ErrorAction SilentlyContinue | ForEach-Object { $cands += , @($_.FullName) }
    }
    foreach ($c in $cands) {
        try {
            $v = & $c[0] @($c | Select-Object -Skip 1) -c "import sys; print(sys.version_info >= (3, 9))" 2>$null
            if ($v -eq "True") { $base = $c; break }
        } catch { }
    }
    if (-not $base) {
        throw "Python 3.9 ou plus est introuvable. L'installer depuis https://www.python.org/downloads/ (cocher « Add python.exe to PATH »), puis relancer."
    }
    Write-Host "Création de l'environnement Python dans $env_dir…"
    & $base[0] @($base | Select-Object -Skip 1) -m venv $env_dir
}
$ok = & $py -c "import frida; print(frida.__version__)" 2>$null
if ($ok -ne $FRIDA) {
    Write-Host "Installation de frida $FRIDA…"
    & $py -m pip install --disable-pip-version-check -q "frida==$FRIDA"
    if ($LASTEXITCODE -ne 0) { throw "Échec de pip install frida." }
}
Write-Host "Python et frida $FRIDA prêts."
if ($Verifier) { return }

# --- Hook --------------------------------------------------------------------------------------
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$nomPropre = ($Nom -replace '[^A-Za-z0-9_-]', '_')
$out = Join-Path $env:TEMP "salon-$nomPropre-$stamp"
New-Item -ItemType Directory -Force $out | Out-Null
Write-Host ""
Write-Host "Journal du hook : $out\frida.log"
Write-Host "Entrée = photo des emplacements. « fin » puis Entrée = fin du hook (pas Ctrl+C)."
Write-Host ""
try {
    & $py -I (Join-Path $ici "hook_lobby.py") $out
} catch { }

# --- Collecte ----------------------------------------------------------------------------------
Write-Host ""
Write-Host "Ferme Civ6 maintenant (sans le relancer : il réécrit ses journaux au démarrage)."
Read-Host "Puis appuie sur Entrée pour créer l'archive"
$local = Join-Path $env:LOCALAPPDATA "Firaxis Games\Sid Meier's Civilization VI"
$logs = New-Item -ItemType Directory -Force (Join-Path $out "Logs")
foreach ($f in "net_message_debug.log", "net_connection_debug.log", "Lua.log", "UserInterface.log",
               "Modding.log", "Database.log", "FiraxisLive.log") {
    Copy-Item (Join-Path $local "Logs\$f") $logs -ErrorAction SilentlyContinue
}
Copy-Item (Join-Path $local "AppOptions.txt") $out -ErrorAction SilentlyContinue
@{
    nom = $Nom
    machine = $env:COMPUTERNAME
    collecte = (Get-Date).ToString("o")
    fuseau = [TimeZoneInfo]::Local.Id
    jeu_ouvert = [bool](Get-Process | Where-Object { $_.Name -like "CivilizationVI*" })
    langue = (Get-Culture).Name
} | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $out "collecte.json")
$zip = Join-Path ([Environment]::GetFolderPath("Desktop")) "salon-$nomPropre-$stamp.zip"
Compress-Archive -Path (Join-Path $out "*") -DestinationPath $zip
Write-Host "Archive écrite : $zip"
