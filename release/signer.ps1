<#
.SYNOPSIS
Signe une release en brouillon de BetterGameExperience (Authenticode), puis refait son manifeste.
Voir release/README.md, « Signature Authenticode ».

.DESCRIPTION
Le workflow de release compile sur GitHub et crée un brouillon « (à signer) ». La clé du
certificat de signature de code est dans un coffre distant, ouvert par une application de bureau
avec un code à usage unique : elle ne peut pas servir dans la CI. Ce script, lancé depuis le PC qui
a accès au certificat :
  1. télécharge les fichiers du brouillon et vérifie le manifeste tel que la CI l'a signé ;
  2. signe les deux DLL et l'installeur (signtool, SHA-256, horodatage RFC 3161) ;
  3. recalcule le manifeste (taille et SHA-256 changent avec la signature) et le re-signe en ed25519 ;
  4. renvoie le tout dans le brouillon et retire « (à signer) » du titre.
Le brouillon reste à relire et publier à la main.

Rien de personnel dans ce fichier. Le certificat est choisi par son empreinte : -Empreinte, sinon
$env:BGE_EMPREINTE_SIGNATURE, sinon %USERPROFILE%\.bgd\empreinte_signature.txt, sinon le seul
certificat de signature de code (avec sa clé) du magasin de l'utilisateur. La clé ed25519 du
manifeste vient de $env:BGD_SIGNING_KEY, sinon de %USERPROFILE%\.bgd\signing_key.txt.

Relancer le script sur un brouillon déjà signé ne re-signe pas les fichiers qui portent déjà une
signature valide du même certificat.

.EXAMPLE
.\release\signer.ps1 v0.12.0
.\release\signer.ps1 -Dossier C:\chemin\vers\artefact      (signe un dossier local, sans GitHub)
#>
[CmdletBinding(DefaultParameterSetName = 'Release')]
param(
	[Parameter(ParameterSetName = 'Release', Position = 0, Mandatory = $true)][string]$Tag,
	[Parameter(ParameterSetName = 'Dossier', Mandatory = $true)][string]$Dossier,
	[string]$Empreinte,
	[string]$Horodatage = 'http://time.certum.pl'
)

$ErrorActionPreference = 'Stop'
try { [Console]::OutputEncoding = [Text.Encoding]::UTF8 } catch {}

$ici = $PSScriptRoot
$manifestPy = Join-Path $ici 'manifest.py'
$dossierBgd = Join-Path $env:USERPROFILE '.bgd'
$suffixeBrouillon = ' (à signer)'
$description = 'BetterGameExperience'

function Stop-Signature([string]$message) {
	Write-Host $message -ForegroundColor Red
	exit 1
}

function Invoke-Natif {
	# Lance un exécutable et s'arrête s'il échoue (PowerShell 5.1 ne le fait pas seul).
	param([string]$Exe, [string[]]$Arguments)
	& $Exe @Arguments
	if ($LASTEXITCODE -ne 0) { Stop-Signature "Échec : $Exe $($Arguments -join ' ')" }
}

function Find-Signtool {
	$cmd = Get-Command signtool.exe -ErrorAction SilentlyContinue
	if ($cmd) { return $cmd.Source }
	$kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
	$trouve = Get-ChildItem $kits -Directory -ErrorAction SilentlyContinue |
		Where-Object { $_.Name -match '^\d+(\.\d+)+$' } |
		Sort-Object { [version]$_.Name } -Descending |
		ForEach-Object { Join-Path $_.FullName 'x64\signtool.exe' } |
		Where-Object { Test-Path $_ } |
		Select-Object -First 1
	if (-not $trouve) {
		Stop-Signature 'signtool introuvable : installer le Windows SDK (« Signing Tools for Desktop Apps » suffit).'
	}
	return $trouve
}

function Get-Certificat {
	$e = $Empreinte
	if (-not $e) { $e = $env:BGE_EMPREINTE_SIGNATURE }
	$fichier = Join-Path $dossierBgd 'empreinte_signature.txt'
	if (-not $e -and (Test-Path $fichier)) { $e = (Get-Content $fichier -Raw).Trim() }
	if ($e) {
		$e = ($e -replace '\s', '').ToUpperInvariant()
		$c = Get-ChildItem Cert:\CurrentUser\My | Where-Object { $_.Thumbprint -eq $e }
		if (-not $c) { Stop-Signature "Certificat $e absent du magasin de l'utilisateur : l'application du coffre est-elle connectée ?" }
	} else {
		$c = @(Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert | Where-Object { $_.HasPrivateKey -and $_.NotAfter -gt (Get-Date) })
		if ($c.Count -eq 0) { Stop-Signature "Aucun certificat de signature de code avec sa clé : l'application du coffre est-elle connectée ?" }
		if ($c.Count -gt 1) {
			$c | ForEach-Object { Write-Host "  $($_.Thumbprint)  $($_.Subject)" }
			Stop-Signature "Plusieurs certificats possibles : en choisir un avec -Empreinte ou $fichier."
		}
		$c = $c[0]
	}
	if (-not $c.HasPrivateKey) { Stop-Signature "Le certificat $($c.Thumbprint) n'est relié à aucune clé." }
	if ($c.NotAfter -le (Get-Date)) { Stop-Signature "Le certificat $($c.Thumbprint) a expiré le $($c.NotAfter)." }
	return $c
}

function Get-CleManifeste {
	if ($env:BGD_SIGNING_KEY) { return $env:BGD_SIGNING_KEY }
	$fichier = Join-Path $dossierBgd 'signing_key.txt'
	if (-not (Test-Path $fichier)) { Stop-Signature "Clé du manifeste absente : ni BGD_SIGNING_KEY ni $fichier." }
	return (Get-Content $fichier -Raw).Trim()
}

function Test-SigneePar([string]$Chemin, $Certificat) {
	$s = Get-AuthenticodeSignature $Chemin
	return ($s.Status -eq 'Valid' -and $s.SignerCertificate.Thumbprint -eq $Certificat.Thumbprint)
}

function Invoke-Signature([string]$Signtool, [string]$Chemin, $Certificat) {
	if (Test-SigneePar $Chemin $Certificat) {
		Write-Host "Déjà signé : $(Split-Path $Chemin -Leaf)"
		return
	}
	# Le serveur d'horodatage refuse parfois une requête : quelques essais avant d'abandonner.
	for ($essai = 1; $essai -le 4; $essai++) {
		& $Signtool sign /sha1 $Certificat.Thumbprint /s My /fd sha256 /tr $Horodatage /td sha256 /d $description /du "https://github.com/$depot" $Chemin
		if ($LASTEXITCODE -eq 0) { break }
		if ($essai -eq 4) { Stop-Signature "Échec de la signature : $Chemin" }
		Write-Host "Nouvel essai dans $(5 * $essai) s..." -ForegroundColor Yellow
		Start-Sleep -Seconds (5 * $essai)
	}
	Invoke-Natif $Signtool @('verify', '/pa', '/q', $Chemin)
	if (-not (Test-SigneePar $Chemin $Certificat)) { Stop-Signature "Signature invalide après coup : $Chemin" }
	Write-Host "Signé : $(Split-Path $Chemin -Leaf)" -ForegroundColor Green
}

# --- Prérequis, avant de toucher à quoi que ce soit ---
$signtool = Find-Signtool
$certificat = Get-Certificat
$cle = Get-CleManifeste
# Dépôt GitHub : celui du remote de ce clone (aussi l'adresse affichée dans la signature).
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { Stop-Signature 'gh (GitHub CLI) introuvable.' }
Push-Location $ici
try { $depot = gh repo view --json nameWithOwner -q .nameWithOwner } finally { Pop-Location }
if ($LASTEXITCODE -ne 0 -or -not $depot) { Stop-Signature 'Dépôt GitHub introuvable (gh repo view).' }
$python = (Get-Command python -ErrorAction SilentlyContinue).Source
if (-not $python) { Stop-Signature 'python introuvable.' }
& $python -c 'import cryptography' 2>$null
if ($LASTEXITCODE -ne 0) { Stop-Signature 'Module Python cryptography absent : pip install cryptography' }
Write-Host "Certificat : $($certificat.Subject) (expire le $($certificat.NotAfter.ToString('d')))"

# --- Fichiers à signer ---
if ($PSCmdlet.ParameterSetName -eq 'Release') {
	$infos = gh release view $Tag --repo $depot --json isDraft,name | ConvertFrom-Json
	if ($LASTEXITCODE -ne 0) { Stop-Signature "Release $Tag introuvable." }
	# Une release publiée est déjà chez les joueurs : changer ses fichiers changerait leurs SHA-256.
	if (-not $infos.isDraft) { Stop-Signature "$Tag est déjà publiée : on ne signe qu'un brouillon." }
	$dist = Join-Path ([IO.Path]::GetTempPath()) "bge_signature_$Tag"
	if (Test-Path $dist) { Remove-Item $dist -Recurse -Force }
	New-Item -ItemType Directory $dist | Out-Null
	Invoke-Natif gh @('release', 'download', $Tag, '--repo', $depot, '--dir', $dist)
} else {
	$dist = (Resolve-Path $Dossier).Path
}

$manifeste = Join-Path $dist 'manifest.json'
foreach ($f in @($manifeste, "$manifeste.sig")) {
	if (-not (Test-Path $f)) { Stop-Signature "Fichier absent : $f" }
}
$dlls = @('BGD_GameCore_XP2.dll', 'winmm.dll') | ForEach-Object { Join-Path $dist $_ }
$installeurs = @(Get-ChildItem $dist -Filter 'BetterGameExperience_V*.exe' | ForEach-Object { $_.FullName })
foreach ($f in $dlls) { if (-not (Test-Path $f)) { Stop-Signature "Fichier absent : $f" } }
if ($installeurs.Count -ne 1) { Stop-Signature "Il faut exactement un BetterGameExperience_V*.exe dans $dist." }

# Le manifeste reçu doit être celui de la CI, intact, avant qu'on le refasse : sinon on signerait des
# fichiers qui ne sont pas ceux du workflow. Sauf si ce brouillon a déjà été signé par ce script.
$dejaSignes = @($dlls | Where-Object { Test-SigneePar $_ $certificat }).Count -eq $dlls.Count
Write-Host 'Vérification du manifeste reçu...'
& $python -I $manifestPy verify $manifeste
if ($LASTEXITCODE -ne 0 -and -not $dejaSignes) { Stop-Signature 'Le manifeste reçu ne correspond pas aux fichiers : rien n''est signé.' }

# --- Signature Authenticode ---
foreach ($f in $dlls + $installeurs) { Invoke-Signature $signtool $f $certificat }

# --- Manifeste refait et re-signé ---
& $python -I $manifestPy rehash $manifeste | Out-Null
if ($LASTEXITCODE -ne 0) { Stop-Signature 'Échec du recalcul du manifeste.' }
$ancienneCle = $env:BGD_SIGNING_KEY
try {
	$env:BGD_SIGNING_KEY = $cle
	Invoke-Natif $python @('-I', $manifestPy, 'sign', $manifeste)
} finally {
	$env:BGD_SIGNING_KEY = $ancienneCle
}
Invoke-Natif $python @('-I', $manifestPy, 'verify', $manifeste)

# --- Retour dans le brouillon ---
if ($PSCmdlet.ParameterSetName -eq 'Release') {
	$envoi = @($dlls + $installeurs + @($manifeste, "$manifeste.sig"))
	Invoke-Natif gh (@('release', 'upload', $Tag, '--repo', $depot, '--clobber') + $envoi)
	if ($infos.name.EndsWith($suffixeBrouillon)) {
		$titre = $infos.name.Substring(0, $infos.name.Length - $suffixeBrouillon.Length)
		Invoke-Natif gh @('release', 'edit', $Tag, '--repo', $depot, '--title', $titre)
	}
	Remove-Item $dist -Recurse -Force
	Write-Host "Brouillon $Tag signé. Il reste à le relire et le publier sur GitHub." -ForegroundColor Green
} else {
	Write-Host "Dossier signé : $dist" -ForegroundColor Green
}
