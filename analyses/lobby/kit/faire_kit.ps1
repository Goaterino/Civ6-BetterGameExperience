# Construit le kit à envoyer aux joueurs du test : Bureau\kit-test-salon.zip
#   powershell -ExecutionPolicy Bypass -File analyses\lobby\kit\faire_kit.ps1
$ErrorActionPreference = "Stop"
$ici = Split-Path -Parent $MyInvocation.MyCommand.Path
$lobby = Split-Path -Parent $ici
$tmp = Join-Path $env:TEMP "kit-test-salon"
Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $tmp | Out-Null
Copy-Item (Join-Path $ici "test_salon.ps1"), (Join-Path $ici "FICHE.md") $tmp
Copy-Item (Join-Path $lobby "hook_lobby.js"), (Join-Path $lobby "hook_lobby.py") $tmp
$zip = Join-Path ([Environment]::GetFolderPath("Desktop")) "kit-test-salon.zip"
Remove-Item $zip -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $tmp "*") -DestinationPath $zip
Remove-Item $tmp -Recurse -Force
Write-Host "Kit écrit : $zip"
