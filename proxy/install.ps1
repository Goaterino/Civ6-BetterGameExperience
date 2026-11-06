# Installe (ou retire avec -Uninstall) le proxy de GameCore_XP2_FinalRelease.dll, posé à côté de
# l'original sous le nom BGD_GameCore_XP2.dll, et écrit ses réglages (rollover_proxy.cfg).
#   powershell -ExecutionPolicy Bypass -File proxy\install.ps1 -Mode count
#   powershell -ExecutionPolicy Bypass -File proxy\install.ps1 -Ameliorations verify
#   powershell -ExecutionPolicy Bypass -File proxy\install.ps1 -Actif off   (installé, mais inactif)
#   powershell -ExecutionPolicy Bypass -File proxy\install.ps1 -Uninstall
# Aucun fichier du jeu n'est remplacé : une vérification des fichiers ou une mise à jour par Steam ne
# retire pas le proxy. Il est chargé par le proxy de démarrage (startup_proxy\install.ps1), à
# installer aussi. Une installation d'avant la version 0.4 (proxy à la place de l'original, original
# sous GameCore_XP2_FinalRelease_orig.dll) est d'abord remise en place, comme le fait l'installeur.
param(
    # Interrupteur général : off, le proxy reste installé mais ne pose aucun détour (GameCore
    # d'origine). Aussi réglable dans l'onglet BetterGameDLLs des options du jeu.
    [ValidateSet("on", "off")]
    [string]$Actif = "on",
    [ValidateSet("coalesce", "count", "off")]
    [string]$Mode = "coalesce",
    # Portée du regroupement des instantanés : cascade de guerres seulement, ou chaque passe de message.
    [ValidateSet("cascade", "message")]
    [string]$Portee = "cascade",
    # Détour de 180260720 (amélioration constructible) : voir src/improvements.rs.
    [ValidateSet("off", "count", "verify", "fast", "quiet")]
    [string]$Ameliorations = "off",
    # Mémoire de 18037ec60 (rendement d'une case) pendant les attentes de l'IA : voir src/rendements.rs.
    [ValidateSet("off", "count", "verify", "fast", "quiet")]
    [string]$Rendements = "off",
    # Accélération de l'affichage pendant la visualisation d'un nuke, si le combat rapide est
    # activé : facteur, ou off. Voir src/wmd.rs.
    [string]$WmdAccel = "20",
    # Boutons d'attaque et de frappe nucléaire des bannières au-dessus des drapeaux d'unités, recentrage
    # de la caméra au clic sur ces boutons, journal des scripts et XML lus. Voir src/ui.rs.
    [ValidateSet("on", "off")]
    [string]$UiBoutons = "on",
    [ValidateSet("on", "off")]
    [string]$UiRecentrage = "off",
    [ValidateSet("on", "off")]
    [string]$UiJournal = "off",
    # Mesure du coût des épingles de carte dans Lua.log (voir map_pins/README.md).
    [ValidateSet("on", "off")]
    [string]$UiMesureEpingles = "off",
    # Diagnostic des accords croisés entre humains dans Lua.log (voir docs/netcode/README.md).
    [ValidateSet("on", "off")]
    [string]$UiDiagAccords = "off",
    # Signaux du chat caché du départ de tour simultané laissés visibles (voir depart_tour/README.md).
    [ValidateSet("on", "off")]
    [string]$DepartSignaux = "off",
    # Épingles de carte : plus de reconstruction inutile à chaque PlayerInfoChanged (voir map_pins/README.md).
    [ValidateSet("on", "off")]
    [string]$UiEpingles = "on",
    # Merveille retirée de la file par un échange : remise en tête par des commandes du jeu, sans
    # désynchro (voir wonder_queue/README.md).
    [ValidateSet("on", "off")]
    [string]$UiMerveilles = "on",
    # Rendements d'empire de l'interface rafraîchis après un gouverneur ou un personnage illustre
    # (voir empire_yields/README.md).
    [ValidateSet("on", "off")]
    [string]$UiRendements = "on",
    # Compteur de tours de production qui inclut les bonus plats de gouvernement (fascisme,
    # autocratie, république marchande). Voir src/tours.rs.
    [ValidateSet("on", "off")]
    [string]$ToursProduction = "on",
    # Mods d'interface repris dans le proxy, prioritaires sur le mod s'il est actif (voir
    # mods_ui/README.md) : charges des bâtisseurs et tours des routes commerciales sur les drapeaux,
    # effacement des notifications, bandeau étendu, effet des cartes de politique, écran des rapports.
    [ValidateSet("on", "off")]
    [string]$UiCharges = "off",
    [ValidateSet("on", "off")]
    [string]$UiRoutes = "off",
    [ValidateSet("on", "off")]
    [string]$UiNotifications = "off",
    [ValidateSet("on", "off")]
    [string]$UiBandeau = "off",
    [ValidateSet("on", "off")]
    [string]$UiPolitiques = "off",
    [ValidateSet("on", "off")]
    [string]$UiRapports = "off",
    # Profil des zones chronométrées de l'exe (diagnostic, lu par le proxy de démarrage).
    [ValidateSet("on", "off")]
    [string]$ProfilZones = "off",
    [switch]$Uninstall,
    [string]$GameDir = "C:\Program Files (x86)\Steam\steamapps\common\Sid Meier's Civilization VI"
)
$ErrorActionPreference = "Stop"

$bin = Join-Path $GameDir "DLC\Expansion2\Binaries\Win64"
$dll = Join-Path $bin "GameCore_XP2_FinalRelease.dll"
$orig = Join-Path $bin "GameCore_XP2_FinalRelease_orig.dll"
$proxy = Join-Path $bin "BGD_GameCore_XP2.dll"
$cfg = Join-Path $bin "rollover_proxy.cfg"
if (-not (Test-Path $dll)) { throw "GameCore_XP2_FinalRelease.dll introuvable dans $bin" }
if (Get-Process | Where-Object { $_.Name -like "CivilizationVI*" }) { throw "Ferme Civilization VI d'abord." }

function Test-Ours($path) {
    if (-not (Test-Path $path)) { return $false }
    [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($path)).Contains("RolloverProxyStatus")
}

# Prototypes précédents, posés à côté de l'exe : on les retire s'ils sont encore là.
$exeDir = Join-Path $GameDir "Base\Binaries\Win64Steam"
foreach ($old in "version.dll", "dxgi.dll") {
    $p = Join-Path $exeDir $old
    if (Test-Ours $p) { Remove-Item $p; Write-Host "Ancien proxy $old retiré." }
}
Remove-Item (Join-Path $exeDir "rollover_proxy.cfg") -ErrorAction SilentlyContinue

# Ancienne installation : l'original revient ; un _orig à côté d'un original remis par Steam est périmé.
if (Test-Ours $dll) {
    if (-not (Test-Path $orig)) { throw "Un ancien proxy est à la place de l'original mais $orig manque : vérifie les fichiers du jeu dans Steam." }
    Remove-Item $dll
    Rename-Item $orig (Split-Path $dll -Leaf)
    Write-Host "Ancienne installation retirée : GameCore_XP2_FinalRelease.dll est de nouveau l'original."
} elseif (Test-Path $orig) {
    Remove-Item $orig
    Write-Host "$orig périmé retiré."
}

if ($Uninstall) {
    if (Test-Ours $proxy) { Remove-Item $proxy }
    Remove-Item $cfg -ErrorAction SilentlyContinue
    Write-Host "Proxy retiré."
    return
}

$built = Join-Path $PSScriptRoot "target\release\BGD_GameCore_XP2.dll"
if (-not (Test-Path $built)) { throw "Compile d'abord : cargo build --release dans $PSScriptRoot" }
Copy-Item $built $proxy -Force
Set-Content -Encoding ascii $cfg "actif=$Actif", "mode=$Mode", "portee=$Portee", "ameliorations=$Ameliorations", "rendements=$Rendements", "wmd_accel=$WmdAccel", "ui_boutons=$UiBoutons", "ui_recentrage=$UiRecentrage", "ui_journal=$UiJournal", "ui_mesure_epingles=$UiMesureEpingles", "ui_diag_accords=$UiDiagAccords", "depart_signaux=$DepartSignaux", "ui_epingles=$UiEpingles", "ui_merveilles=$UiMerveilles", "ui_rendements=$UiRendements", "tours_production=$ToursProduction", "ui_charges=$UiCharges", "ui_routes=$UiRoutes", "ui_notifications=$UiNotifications", "ui_bandeau=$UiBandeau", "ui_politiques=$UiPolitiques", "ui_rapports=$UiRapports", "profil_zones=$ProfilZones"
Write-Host "Proxy installé dans $bin (actif $Actif, mode $Mode, portée $Portee, ameliorations $Ameliorations, rendements $Rendements, wmd_accel $WmdAccel, ui_boutons $UiBoutons, ui_recentrage $UiRecentrage, ui_journal $UiJournal, ui_mesure_epingles $UiMesureEpingles, ui_diag_accords $UiDiagAccords, depart_signaux $DepartSignaux, ui_epingles $UiEpingles, ui_merveilles $UiMerveilles, ui_rendements $UiRendements, tours_production $ToursProduction, ui_charges $UiCharges, ui_routes $UiRoutes, ui_notifications $UiNotifications, ui_bandeau $UiBandeau, ui_politiques $UiPolitiques, ui_rapports $UiRapports, profil_zones $ProfilZones) ; chargé par le proxy de démarrage (startup_proxy/install.ps1)."
Write-Host "Journal : $env:LOCALAPPDATA\Civ6RolloverProxy\RolloverProxy.log"
