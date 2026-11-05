# Capture WPR d'un passage de tour de Civ6 : CPU échantillonné avec piles, changements de contexte,
# présentations DXGI (temps d'image) et trafic réseau par processus (civ6.wprp), et en parallèle la
# charge du GPU (compteurs « GPU Engine » par processus, nvidia-smi dmon s'il existe : consommation,
# fréquences).
# À lancer dans un PowerShell administrateur, jeu ouvert, juste avant de finir le tour :
#   powershell -ExecutionPolicy Bypass -File rollover\capture.ps1 -Name tour79
# Scénario à durée fixe (tour du joueur, caméra immobile) : -Duration 60 arrête seul la capture.
# Toute une partie sans gêner le jeu : -Light (images, réseau, GPU ; ni échantillons CPU ni noyau).
# Puis analyser (sans droits admin) :
#   .venv\Scripts\python.exe -I rollover\analyze.py rollover\captures\<dossier>
param(
    [string]$Name = "passage",
    # Ajoute les E/S disque et fichier, pour voir le coût de la sauvegarde auto.
    [switch]$WithIO,
    # Durée en secondes : la capture s'arrête seule, sans attendre Entrée.
    [int]$Duration = 0,
    # Capture légère, à laisser tourner une partie entière : seulement le profil Frames de civ6.wprp
    # (présentations DXGI, trafic réseau). L'échantillonnage CPU avec piles fait ramer le jeu.
    [switch]$Light
)
$ErrorActionPreference = "Stop"

$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) { throw "WPR a besoin d'un PowerShell lancé en administrateur." }
$game = Get-Process | Where-Object { $_.Name -like "CivilizationVI*" } | Select-Object -First 1
if (-not $game) { throw "Civilization VI ne tourne pas." }

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$dir = Join-Path $PSScriptRoot "captures\$stamp-$Name"
New-Item -ItemType Directory -Force $dir | Out-Null
$etl = Join-Path $dir "trace.etl"

# Une capture déjà en cours (session interrompue) empêcherait le démarrage. Sans capture en cours,
# wpr -cancel écrit sur stderr, ce que PowerShell 5.1 transformerait en erreur fatale : on passe par cmd.
cmd /c "wpr -cancel >nul 2>&1"
$profiles = @("-start", "$(Join-Path $PSScriptRoot 'civ6.wprp')!Frames")
if (-not $Light) { $profiles = @("-start", "CPU") + $profiles }
if ($WithIO) { $profiles += @("-start", "DiskIO", "-start", "FileIO") }
wpr @profiles -filemode
if ($LASTEXITCODE -ne 0) { throw "wpr n'a pas démarré (code $LASTEXITCODE)." }

# Charge du GPU, une mesure par seconde. Les instances « GPU Engine » portent le pid : analyze.py
# garde celles du jeu. Les deux outils écrivent ligne à ligne ; on les arrête avec la capture.
$samplers = @()
$samplers += Start-Process typeperf -PassThru -WindowStyle Hidden -ArgumentList @(
    '"\GPU Engine(*)\Utilization Percentage"', "-si", "1", "-f", "CSV", "-y",
    "-o", "`"$(Join-Path $dir 'gpu_moteurs.csv')`"")
if (Get-Command nvidia-smi -ErrorAction SilentlyContinue) {
    # Avec -f, dmon n'écrit rien s'il est arrêté de force : on redirige sa sortie standard.
    $samplers += Start-Process nvidia-smi -PassThru -WindowStyle Hidden -ArgumentList @("dmon", "-s", "puc", "-o", "DT") `
        -RedirectStandardOutput (Join-Path $dir "gpu_nvidia.txt")
}

$t0 = Get-Date
Write-Host ""
if ($Duration -gt 0) {
    Write-Host "Capture en cours pendant $Duration s." -ForegroundColor Yellow
    Start-Sleep -Seconds $Duration
} elseif ($Light) {
    Write-Host "Capture légère en cours. Joue la partie, puis appuie sur Entrée ici à la fin," -ForegroundColor Yellow
    Write-Host "avant de quitter le jeu." -ForegroundColor Yellow
    [void](Read-Host)
} else {
    Write-Host "Capture en cours. Termine le tour dans le jeu, attends d'avoir de nouveau la main,"
    Write-Host "puis appuie sur Entrée ici." -ForegroundColor Yellow
    [void](Read-Host)
}
$t1 = Get-Date
$samplers | Stop-Process -ErrorAction SilentlyContinue
wpr -stop $etl "Civ6 passage de tour $Name"
if ($LASTEXITCODE -ne 0) { throw "wpr n'a pas écrit la trace (code $LASTEXITCODE)." }

# Les journaux du jeu sont réécrits à chaque lancement : on garde la copie de cette session.
$logs = Join-Path $env:LOCALAPPDATA "Firaxis Games\Sid Meier's Civilization VI\Logs"
foreach ($f in "Profile.csv", "net_message_debug.log", "net_connection_debug.log", "Lua.log", "Modding.log") {
    Copy-Item (Join-Path $logs $f) $dir -ErrorAction SilentlyContinue
}
# Réglages graphiques et threads du jeu (MaxJobThreads…), et ceux du proxy : deux captures ne se
# comparent qu'à réglages égaux.
foreach ($f in "AppOptions.txt", "GraphicsOptions.txt") {
    Copy-Item (Join-Path (Split-Path $logs) $f) $dir -ErrorAction SilentlyContinue
}
if ($game.Path) {  # <jeu>\Base\Binaries\Win64Steam\CivilizationVI*.exe
    $gameDir = Split-Path (Split-Path (Split-Path (Split-Path $game.Path)))
    Copy-Item (Join-Path $gameDir "DLC\Expansion2\Binaries\Win64\rollover_proxy.cfg") $dir -ErrorAction SilentlyContinue
}
# Zones du profileur de l'exe (proxy de démarrage, profil_zones=on) : le fichier de cette session du
# jeu, que le proxy écrit encore (il le partage en lecture).
Get-ChildItem (Join-Path $env:LOCALAPPDATA "Civ6RolloverProxy\zones\zones-$($game.Id)-*.bin") -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime | Select-Object -Last 1 | Copy-Item -Destination $dir
# Dernière sauvegarde automatique, pour comparer l'état de jeu entre configurations (compare_saves.py).
$saves = Join-Path ([Environment]::GetFolderPath("MyDocuments")) "My Games\Sid Meier's Civilization VI\Saves"
Get-ChildItem (Join-Path $saves "*\auto\AutoSave_*.Civ6Save") -ErrorAction SilentlyContinue |
    Where-Object { $_.LastWriteTime -ge $t0 } | Sort-Object LastWriteTime | Select-Object -Last 1 |
    Copy-Item -Destination $dir
@{ name = $Name; start = $t0.ToString("o"); stop = $t1.ToString("o"); with_io = [bool]$WithIO
   duration = $Duration; pid = $game.Id; exe = $game.Name; light = [bool]$Light
   steam_pid = (Get-Process steam -ErrorAction SilentlyContinue | Select-Object -First 1).Id } |
    ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $dir "capture.json")
Write-Host "Capture écrite dans $dir"
