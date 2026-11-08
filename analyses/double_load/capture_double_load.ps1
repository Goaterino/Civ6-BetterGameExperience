# Capture d'un double chargement à l'entrée en partie (voir README.md du même dossier).
# Autonome : on peut l'envoyer tel quel à l'hôte et au client. Chacun le lance AVANT de lancer
# Civ6 (ou au plus tard dans le salon), puis rejoue la partie jusqu'au double chargement / plantage :
#   powershell -ExecutionPolicy Bypass -File capture_double_load.ps1 -Role hote
#   powershell -ExecutionPolicy Bypass -File capture_double_load.ps1 -Role client
#
# Ce que fait le script, jusqu'à ce qu'on appuie sur Q :
#  - suit en direct les journaux réseau et Lua du jeu, et horodate chaque ligne à la milliseconde
#    (le jeu n'écrit que des secondes) ; les lignes clés vont aussi dans evenements.log ;
#  - relève chaque seconde l'état du processus : CPU, mémoire, « ne répond pas » (thread principal
#    bloqué, par exemple pendant la phase synchrone du chargement) ;
#  - à chaque fermeture ou plantage du jeu, copie aussitôt tout le dossier Logs (le jeu l'écrase au
#    démarrage suivant) et note le code de sortie ;
#  - à la fin : journal d'événements Windows (plantages), dumps récents, écart d'horloge NTP (pour
#    aligner l'hôte et le client), puis une archive zip sur le Bureau à renvoyer.
#
# Touches pendant la capture : M = marqueur horodaté (« là je vois le second chargement »),
#                              N = marqueur avec une note, Q = terminer et créer l'archive.
param(
    [ValidateSet("hote", "client", "")]
    [string]$Role = "",
    # Ne pas joindre les fichiers .dmp (peuvent être gros).
    [switch]$NoDumps,
    # Ne pas mesurer l'écart d'horloge (requête NTP vers time.windows.com).
    [switch]$NoNtp,
    # Arrêt automatique après ce nombre de minutes (0 = seulement avec Q).
    [double]$Minutes = 0
)
$ErrorActionPreference = "Stop"

while ($Role -notin "hote", "client") {
    $Role = (Read-Host "Rôle de cette machine (hote / client)").Trim().ToLower()
}

$local = Join-Path $env:LOCALAPPDATA "Firaxis Games\Sid Meier's Civilization VI"
$logs = Join-Path $local "Logs"
$docs = Join-Path ([Environment]::GetFolderPath("MyDocuments")) "My Games\Sid Meier's Civilization VI"
$proxyLog = Join-Path $env:LOCALAPPDATA "Civ6RolloverProxy\RolloverProxy.log"
$start = Get-Date
$stamp = $start.ToString("yyyyMMdd-HHmmss")
$name = "civ6-doublechargement-$Role-$env:COMPUTERNAME-$stamp"
$out = Join-Path $env:TEMP $name
New-Item -ItemType Directory -Force $out | Out-Null
New-Item -ItemType Directory -Force (Join-Path $out "suivi") | Out-Null

function Now { (Get-Date).ToString("HH:mm:ss.fff") }

function New-Writer([string]$path) {
    $w = New-Object IO.StreamWriter($path, $true, (New-Object Text.UTF8Encoding($false)))
    $w.AutoFlush = $true
    return $w
}

$events = New-Writer (Join-Path $out "evenements.log")
function Write-Event([string]$source, [string]$text, [string]$color = "Gray") {
    $line = "$(Now)  [$source]  $text"
    $events.WriteLine($line)
    Write-Host $line -ForegroundColor $color
}

# --- Suivi des journaux ------------------------------------------------------------------------
# Lignes recopiées aussi dans evenements.log (avec leur couleur à l'écran).
$keyPatterns = @(
    @{ re = "ignored snapshot|full snapshot|Requesting snapshot data|NetSnapshot :|NetSnapshotRequest|NetSnapshotProcessed|PlayerSnapshotStatus"; color = "Yellow" },
    @{ re = "LoadInProgress|Queuing Messages|Flushing Incoming|TurnSynchronizationBarrier|SyncCheck|Resync|Repair|\bOOS\b|desync"; color = "Cyan" },
    @{ re = "PlayerConnected|PlayerDisconnect|Dropped|leaving game|joining|NetRegister|Host migration|timed? ?out|kick"; color = "Magenta" },
    @{ re = "LoadScreen|Loading|LoadGame|SaveGame|BGD|BGE|Error|Exception|assert|crash"; color = "Green" }
)

$tails = @()
foreach ($f in "net_message_debug.log", "net_connection_debug.log", "net_transport_debug.log",
               "GameCore.log", "Lua.log", "UserInterface.log", "Engine.log",
               "LoadGameViewState.log") {
    $tails += [pscustomobject]@{ Name = $f; Path = (Join-Path $logs $f); Pos = [long]-1; Head = ""; Rest = ""
                                Writer = (New-Writer (Join-Path $out "suivi\$f")) }
}
$tails += [pscustomobject]@{ Name = "RolloverProxy.log"; Path = $proxyLog; Pos = [long]-1; Head = ""; Rest = ""
                            Writer = (New-Writer (Join-Path $out "suivi\RolloverProxy.log")) }

function Read-Head($fs) {
    $fs.Seek(0, "Begin") | Out-Null
    $b = New-Object byte[] 64
    $n = $fs.Read($b, 0, 64)
    return [Text.Encoding]::ASCII.GetString($b, 0, $n)
}

function Poll-Tail($t) {
    if (-not (Test-Path -LiteralPath $t.Path)) { return }
    try { $fs = [IO.File]::Open($t.Path, "Open", "Read", "ReadWrite, Delete") } catch { return }
    try {
        $len = $fs.Length
        $head = Read-Head $fs
        if ($t.Pos -lt 0) {
            # Premier passage : on part de la fin, le contenu existant sera dans les copies complètes.
            $t.Pos = $len; $t.Head = $head
            $t.Writer.WriteLine("$(Now)  --- début du suivi à l'octet $len ---")
            return
        }
        $rewritten = ($len -lt $t.Pos) -or ($t.Head.Length -gt 0 -and $head.Length -ge $t.Head.Length -and
                      -not $head.StartsWith($t.Head)) -or ($t.Head.Length -eq 0 -and $len -gt 0 -and $t.Pos -gt 0)
        if ($rewritten) {
            $t.Writer.WriteLine("$(Now)  --- fichier réécrit par le jeu (nouvelle session) ---")
            Write-Event $t.Name "fichier réécrit (le jeu a redémarré sa journalisation)" "DarkGray"
            $t.Pos = 0; $t.Rest = ""
        }
        if ($head.Length -gt $t.Head.Length) { $t.Head = $head }
        if ($len -le $t.Pos) { return }
        $n = [int][Math]::Min($len - $t.Pos, 16MB)
        $buf = New-Object byte[] $n
        $fs.Seek($t.Pos, "Begin") | Out-Null
        $read = $fs.Read($buf, 0, $n)
        $t.Pos += $read
        $text = $t.Rest + [Text.Encoding]::UTF8.GetString($buf, 0, $read)
        $lines = $text -split "`n"
        $t.Rest = $lines[-1]
        $ts = Now
        for ($i = 0; $i -lt $lines.Count - 1; $i++) {
            $l = $lines[$i].TrimEnd("`r")
            $t.Writer.WriteLine("$ts  $l")
            foreach ($p in $keyPatterns) {
                if ($l -match $p.re) {
                    $short = if ($l.Length -gt 220) { $l.Substring(0, 220) + "…" } else { $l }
                    Write-Event $t.Name $short $p.color
                    break
                }
            }
        }
    } finally { $fs.Dispose() }
}

# --- Copies ------------------------------------------------------------------------------------
function Copy-Shared([string]$src, [string]$dst) {
    try {
        $in = [IO.File]::Open($src, "Open", "Read", "ReadWrite, Delete")
        try {
            $o = [IO.File]::Create($dst)
            try { $in.CopyTo($o) } finally { $o.Dispose() }
        } finally { $in.Dispose() }
    } catch { Write-Event "copie" "échec pour $src : $($_.Exception.Message)" "Red" }
}

$session = 0
function Save-Logs([string]$label) {
    $script:session++
    $dir = Join-Path $out ("journaux\{0:D2}-{1}" -f $script:session, $label)
    New-Item -ItemType Directory -Force (Join-Path $dir "Logs") | Out-Null
    if (Test-Path $logs) {
        Get-ChildItem $logs -File | ForEach-Object { Copy-Shared $_.FullName (Join-Path $dir "Logs\$($_.Name)") }
    }
    if (Test-Path $proxyLog) { Copy-Shared $proxyLog (Join-Path $dir "RolloverProxy.log") }
    Write-Event "copie" "dossier Logs copié dans journaux\$(Split-Path $dir -Leaf)" "White"
}

# --- Horloge -----------------------------------------------------------------------------------
function Measure-Clock([string]$when) {
    if ($NoNtp) { return }
    try {
        $r = & w32tm /stripchart /computer:time.windows.com /samples:3 /dataonly 2>&1 | Out-String
        Add-Content -Encoding utf8 (Join-Path $out "horloge.txt") "=== $when ($(Get-Date -Format o))`r`n$r"
    } catch { }
}

# --- Démarrage ---------------------------------------------------------------------------------
Measure-Clock "début"
@{
    role = $Role
    machine = $env:COMPUTERNAME
    debut = $start.ToString("o")
    fuseau = [TimeZoneInfo]::Local.Id
    utc_offset = [TimeZoneInfo]::Local.GetUtcOffset($start).ToString()
    coeurs = [Environment]::ProcessorCount
    os = [Environment]::OSVersion.VersionString
} | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $out "machine.json")
foreach ($f in "AppOptions.txt", "Mods.sqlite") {
    if (Test-Path (Join-Path $local $f)) { Copy-Shared (Join-Path $local $f) (Join-Path $out $f) }
}
foreach ($t in $tails) { Poll-Tail $t }

$procCsv = New-Writer (Join-Path $out "processus.csv")
$procCsv.WriteLine("heure;pid;cpu_pct;working_set_mo;prive_mo;threads;repond")
$proc = $null; $lastCpu = 0.0; $lastSample = Get-Date; $wasResponding = $true
$gamePaths = @()
$marks = 0
$savedSinceExit = $true

Write-Host ""
Write-Host "Capture « $Role » démarrée. Dossier de travail : $out" -ForegroundColor White
Write-Host "Touches : M = marqueur, N = marqueur avec note, Q = terminer et créer l'archive." -ForegroundColor White
Write-Host ""
Write-Event "capture" "début, rôle $Role, machine $env:COMPUTERNAME" "White"

function Find-Game {
    Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.Name -like "CivilizationVI*" } |
        Select-Object -First 1
}

$quit = $false
$tick = 0
try {
    while (-not $quit) {
        foreach ($t in $tails) { Poll-Tail $t }

        # Processus, une fois par seconde (5 tours de 200 ms).
        if ($tick % 5 -eq 0) {
            if ($null -eq $proc) {
                $p = Find-Game
                if ($p) {
                    $proc = $p
                    try { $null = $proc.Handle } catch { }   # garde le handle pour lire le code de sortie
                    $path = try { $proc.Path } catch { "?" }
                    if ($path -and $path -ne "?" -and $gamePaths -notcontains $path) { $gamePaths += $path }
                    $lastCpu = try { $proc.TotalProcessorTime.TotalMilliseconds } catch { 0.0 }
                    $lastSample = Get-Date; $wasResponding = $true; $savedSinceExit = $false
                    Write-Event "jeu" "démarré : PID $($proc.Id), $path" "White"
                }
            } else {
                $exited = $true
                try { $proc.Refresh(); $exited = $proc.HasExited } catch { }
                if ($exited) {
                    $code = try { "0x{0:X8}" -f $proc.ExitCode } catch { "inconnu" }
                    $color = if ($code -eq "0x00000000") { "White" } else { "Red" }
                    Write-Event "jeu" "fermé : PID $($proc.Id), code de sortie $code" $color
                    foreach ($t in $tails) { Poll-Tail $t }
                    Save-Logs ("pid{0}-sortie{1}" -f $proc.Id, $code)
                    $savedSinceExit = $true
                    $proc = $null
                } else {
                    $now = Get-Date
                    $cpu = try { $proc.TotalProcessorTime.TotalMilliseconds } catch { $lastCpu }
                    $dt = ($now - $lastSample).TotalMilliseconds
                    $pct = if ($dt -gt 0) { [Math]::Round(100 * ($cpu - $lastCpu) / ($dt * [Environment]::ProcessorCount), 1) } else { 0 }
                    $lastCpu = $cpu; $lastSample = $now
                    $resp = try { $proc.Responding } catch { $true }
                    $procCsv.WriteLine(("{0};{1};{2};{3};{4};{5};{6}" -f (Now), $proc.Id, $pct,
                        [Math]::Round($proc.WorkingSet64 / 1MB), [Math]::Round($proc.PrivateMemorySize64 / 1MB),
                        $proc.Threads.Count, [int]$resp))
                    if ($resp -ne $wasResponding) {
                        if ($resp) { Write-Event "jeu" "répond de nouveau" "DarkYellow" }
                        else { Write-Event "jeu" "NE RÉPOND PLUS (thread principal bloqué)" "DarkYellow" }
                        $wasResponding = $resp
                    }
                }
            }
        }

        # Clavier.
        try {
            while ([Console]::KeyAvailable) {
                $k = [Console]::ReadKey($true)
                switch ($k.Key) {
                    "M" { $marks++; Write-Event "MARQUE" "marqueur $marks" "White" }
                    "N" {
                        $marks++
                        $note = Read-Host "Note"
                        Write-Event "MARQUE" "marqueur $marks : $note" "White"
                    }
                    "Q" { $quit = $true }
                }
            }
        } catch { }

        if ($Minutes -gt 0 -and ((Get-Date) - $start).TotalMinutes -ge $Minutes) { $quit = $true }
        $tick++
        Start-Sleep -Milliseconds 200
    }
} finally {
    Write-Event "capture" "fin demandée, finalisation…" "White"
    foreach ($t in $tails) { try { Poll-Tail $t } catch { } }
    if (-not $savedSinceExit -or $session -eq 0) {
        $label = if ($proc) { "pid$($proc.Id)-en-cours" } else { "fin" }
        Save-Logs $label
    }

    # Plantages vus par Windows (erreur d'application, WER) depuis le début de la capture.
    try {
        Get-WinEvent -FilterHashtable @{ LogName = "Application"; Id = 1000, 1001, 1002; StartTime = $start } -ErrorAction Stop |
            Where-Object { $_.Message -match "Civilization" } |
            ForEach-Object { "=== $($_.TimeCreated.ToString('o'))  Id=$($_.Id)  $($_.ProviderName)`r`n$($_.Message)`r`n" } |
            Set-Content -Encoding utf8 (Join-Path $out "evenements_windows.txt")
    } catch { "aucun événement ($($_.Exception.Message))" | Set-Content -Encoding utf8 (Join-Path $out "evenements_windows.txt") }

    # Dumps écrits depuis le début de la capture.
    if (-not $NoDumps) {
        $dirs = @((Join-Path $env:LOCALAPPDATA "CrashDumps"), $local, $docs, $env:TEMP,
                  (Join-Path $env:LOCALAPPDATA "Microsoft\Windows\WER\ReportArchive"),
                  (Join-Path $env:LOCALAPPDATA "Microsoft\Windows\WER\ReportQueue"))
        foreach ($gp in $gamePaths) { $d = Split-Path $gp; $dirs += $d; $dirs += (Split-Path $d) }
        $dumpDir = Join-Path $out "dumps"
        foreach ($d in ($dirs | Select-Object -Unique)) {
            if (-not (Test-Path $d)) { continue }
            Get-ChildItem $d -Recurse -Depth 3 -File -Include *.dmp, *.mdmp, Report.wer -ErrorAction SilentlyContinue |
                Where-Object { $_.LastWriteTime -ge $start -and $_.FullName -notlike "$out*" } |
                ForEach-Object {
                    New-Item -ItemType Directory -Force $dumpDir | Out-Null
                    Copy-Shared $_.FullName (Join-Path $dumpDir ("{0}_{1}" -f $_.Directory.Name, $_.Name))
                    Write-Event "dump" "$($_.FullName) ($([Math]::Round($_.Length / 1MB, 1)) Mo)" "Red"
                }
        }
    }

    Measure-Clock "fin"

    # Résumé : les lignes qui tranchent entre les scénarios du README.
    $resume = Join-Path $out "resume.txt"
    Get-ChildItem (Join-Path $out "journaux") -Directory -ErrorAction SilentlyContinue | ForEach-Object {
        $f = Join-Path $_.FullName "Logs\net_message_debug.log"
        Add-Content -Encoding utf8 $resume "===== $($_.Name)"
        if (Test-Path $f) {
            Select-String -LiteralPath $f -Pattern "NetSnapshot|ignored snapshot|full snapshot|Requesting snapshot data|TurnSynchronizationBarrier|Queuing Messages|SyncCheck|PlayerDisconnect|leaving game" |
                ForEach-Object { Add-Content -Encoding utf8 $resume ("{0,6}: {1}" -f $_.LineNumber, $_.Line) }
        } else { Add-Content -Encoding utf8 $resume "(pas de net_message_debug.log)" }
    }

    Write-Event "capture" "terminée" "White"
    foreach ($t in $tails) { $t.Writer.Dispose() }
    $procCsv.Dispose(); $events.Dispose()

    $zip = Join-Path ([Environment]::GetFolderPath("Desktop")) "$name.zip"
    try {
        Compress-Archive -Path (Join-Path $out "*") -DestinationPath $zip -Force
        Write-Host ""
        Write-Host "Archive écrite : $zip" -ForegroundColor Green
        Write-Host "Envoie ce fichier. (Copie de travail laissée dans $out)" -ForegroundColor Green
    } catch {
        Write-Host "Échec de la création de l'archive : $($_.Exception.Message)" -ForegroundColor Red
        Write-Host "Envoie plutôt le dossier $out" -ForegroundColor Red
    }
}
