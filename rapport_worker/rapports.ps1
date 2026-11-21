<#
.SYNOPSIS
Liste et récupère les rapports de bug de BetterGameExperience, par les routes /admin du relais
(rapport_worker). Voir rapport_worker/ANALYSE.md.

.DESCRIPTION
Sans argument : les derniers rapports, du plus récent au plus ancien.
Avec des numéros (ou -Derniers N) : télécharge chaque zip dans rapports\<id>.zip à la racine du
dépôt (dossier ignoré par git), l'extrait dans rapports\<id>\ et affiche son résumé.

L'adresse du relais vient de $env:BGE_RAPPORT_URL, sinon de la variable GitHub BGE_RAPPORT_URL
(gh). La clé de l'équipe vient de $env:BGE_CLE_ADMIN, sinon de rapport_worker\.cle_admin (ignoré par
git) ; -NouvelleCle la crée ou la renouvelle (secret CLE_ADMIN du Worker, par wrangler).

.EXAMPLE
.\rapport_worker\rapports.ps1
.\rapport_worker\rapports.ps1 -Liste 50
.\rapport_worker\rapports.ps1 -Derniers 2
.\rapport_worker\rapports.ps1 BGE-331EFB BGE-EFD552
.\rapport_worker\rapports.ps1 -NouvelleCle
#>
[CmdletBinding()]
param(
	[Parameter(Position = 0, ValueFromRemainingArguments = $true)][string[]]$Id,
	[int]$Liste = 15,
	[int]$Derniers = 0,
	[switch]$NouvelleCle,
	[switch]$Force
)

$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
try { [Console]::OutputEncoding = [Text.Encoding]::UTF8 } catch {}

$ici = $PSScriptRoot
$racine = Split-Path $ici -Parent
$dossier = Join-Path $racine 'rapports'
$fichierCle = Join-Path $ici '.cle_admin'

function Get-AdresseRelais {
	$u = $env:BGE_RAPPORT_URL
	if (-not $u) {
		Push-Location $racine
		try { $u = & gh variable get BGE_RAPPORT_URL 2>$null } finally { Pop-Location }
	}
	if (-not $u) { throw "Adresse du relais introuvable : définir BGE_RAPPORT_URL, ou se connecter avec gh (gh auth login)." }
	return ([string]$u).Trim() -replace '/rapport/?$', ''
}

function Get-CleAdmin {
	if ($env:BGE_CLE_ADMIN) { return $env:BGE_CLE_ADMIN.Trim() }
	if (Test-Path $fichierCle) { return (Get-Content $fichierCle -Raw).Trim() }
	throw "Clé de l'équipe introuvable : lancer .\rapport_worker\rapports.ps1 -NouvelleCle (ou définir BGE_CLE_ADMIN)."
}

function New-CleAdmin {
	$octets = New-Object byte[] 32
	[Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($octets)
	$cle = -join ($octets | ForEach-Object { $_.ToString('x2') })
	# npx.cmd plutôt que npx.ps1 : sous Stop, l'enveloppe PowerShell fait des notices de npm des erreurs.
	Push-Location $ici
	$avant = $ErrorActionPreference
	$ErrorActionPreference = 'Continue'
	try { $cle | & npx.cmd --yes wrangler@4 secret put CLE_ADMIN } finally { $ErrorActionPreference = $avant; Pop-Location }
	if ($LASTEXITCODE -ne 0) { throw "wrangler secret put CLE_ADMIN a échoué ($LASTEXITCODE) : clé locale inchangée." }
	[IO.File]::WriteAllText($fichierCle, $cle)
	Write-Host "Clé de l'équipe mise à jour : secret CLE_ADMIN du Worker et $fichierCle."
}

function Invoke-Admin([string]$chemin, [string]$sortie) {
	$p = @{ Uri = "$script:adresse$chemin"; Headers = @{ Authorization = "Bearer $script:cle" }; UseBasicParsing = $true }
	try {
		if ($sortie) { Invoke-WebRequest @p -OutFile $sortie | Out-Null } else { Invoke-RestMethod @p }
	} catch {
		$code = $_.Exception.Response.StatusCode.value__
		if ($code -eq 404) { throw "404 sur $chemin : numéro inconnu, ou clé refusée (CLE_ADMIN du Worker différente de la clé locale ?)." }
		throw
	}
}

function Show-Liste([int]$n) {
	$l = Invoke-Admin "/admin/rapports?n=$n"
	"{0} rapport(s) dans le bucket, {1:N0} Mo" -f $l.total, ($l.octets / 1MB)
	$l.rapports | ForEach-Object {
		$d = [string]$_.description
		if ($d.Length -gt 70) { $d = $d.Substring(0, 70) + '…' }
		[pscustomobject]@{
			Id          = $_.id
			Recu        = ([datetime]$_.recu).ToLocalTime().ToString('yyyy-MM-dd HH:mm')
			Mo          = [math]::Round($_.octets / 1MB, 1)
			Joueur      = $_.joueur
			Tour        = $_.tour
			Hote        = $_.hote
			Humains     = $_.humains
			BGE         = if ($_.bge) { $_.bge } else { $_.version }
			Description = $d
		}
	} | Format-Table -AutoSize | Out-String -Width 250
}

function Show-Resume([string]$id, [string]$cible) {
	$m = Get-Content (Join-Path $cible 'meta.json') -Raw -Encoding UTF8 | ConvertFrom-Json
	$date = if ($m.date_unix) { [DateTimeOffset]::FromUnixTimeSeconds([long]$m.date_unix).UtcDateTime.ToString('yyyy-MM-dd HH:mm:ss') + ' UTC' } else { '?' }
	$partie = if ($m.multijoueur -eq '1') { "multijoueur, $($m.humains) humain(s)" + $(if ($m.hote -eq '1') { ', hôte' } else { '' }) + $(if ($m.en_pause -eq '1') { ', en pause' } else { '' }) } else { 'solo' }
	''
	"== $id  ($date)"
	"Joueur      : $($m.joueur) (id $($m.joueur_id), Steam $($m.steam_id)), $($m.civilisation), $($m.dirigeant)"
	"Partie      : tour $($m.tour), $partie"
	"BGE         : $($m.bge_release) ($($m.bge_version)), $($m.exe), langue $($m.langue)"
	"Description : $($m.description)"
	$lua = Join-Path $cible 'logs\Lua.log'
	if (Test-Path $lua) {
		$graines = Select-String -Path $lua -Pattern 'Map Seed = (-?\d+)' | ForEach-Object { $_.Matches[0].Groups[1].Value }
		if ($graines) { "Graines     : $($graines -join ', ') (la dernière est la partie du rapport)" }
	}
	"Dossier     : $cible"
}

function Get-Rapport([string]$id) {
	$zip = Join-Path $dossier "$id.zip"
	$cible = Join-Path $dossier $id
	if ($Force -or -not (Test-Path $zip)) { Invoke-Admin "/admin/rapport/$id" $zip }
	if ($Force -and (Test-Path $cible)) { Remove-Item $cible -Recurse -Force }
	if (-not (Test-Path $cible)) {
		Add-Type -AssemblyName System.IO.Compression.FileSystem
		# ExtractToDirectory refuse les entrées qui sortiraient du dossier.
		[IO.Compression.ZipFile]::ExtractToDirectory($zip, $cible)
	}
	Show-Resume $id $cible
}

if ($NouvelleCle) { New-CleAdmin; return }

$script:adresse = Get-AdresseRelais
$script:cle = Get-CleAdmin

$ids = @($Id | Where-Object { $_ } | ForEach-Object { $_.Trim().ToUpperInvariant() })
if ($Derniers -gt 0) {
	$ids += @((Invoke-Admin "/admin/rapports?n=$Derniers").rapports | ForEach-Object { $_.id })
}
if ($ids.Count -eq 0) { Show-Liste $Liste; return }

foreach ($i in $ids) {
	if ($i -notmatch '^BGE-[0-9A-F]{6}$') { Write-Warning "$i : pas un numéro de rapport (BGE-XXXXXX)"; continue }
	New-Item -ItemType Directory -Force $dossier | Out-Null
	Get-Rapport $i
}
