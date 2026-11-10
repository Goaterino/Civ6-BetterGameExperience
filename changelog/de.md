# Änderungsprotokoll

[English](../CHANGELOG.md) · weitere Sprachen: siehe das [englische Protokoll](../CHANGELOG.md).

## BetterGameExperience 0.8.2

### Neu
- **Fortschritt von Forschung und Ausrichtung**: Im Panel oben links zeigt **x/y** neben dem Namen
  der aktuellen Technologie und Ausrichtung die bereits investierte Wissenschaft (Kultur) und die
  Gesamtkosten, gefolgt von **(+z)**, wenn das Spiel Wissenschaft (Kultur) aus einer mit mehr als
  ihren Kosten abgeschlossenen Technologie (Ausrichtung) als Überschuss behält. Eine Einstellung
  schaltet es ab.

## BetterGameExperience 0.8.1

### Neu
- **Produktion im Stadtbanner abbrechen**: Ein Rechtsklick auf das Produktionssymbol im Banner
  einer eurer Städte entfernt das aktuelle Element, wie im Produktionsfenster.
- **Vergünstigte Bezirke**: In der Produktionsliste zeigt ein Bezirk mit gesenkten Kosten (weniger
  Bezirke dieses Typs als euer Durchschnitt) seinen Rabatt an, zum Beispiel **-40%**. Die Anzeige
  wird aktualisiert, sobald ein Bezirk platziert wird, denn ein neuer Bezirk kann den Rabatt
  beenden. Bezirke, die nicht in den Durchschnitt eingehen, wie das Thành Vietnams, werden auch
  angezeigt.
- **Gespeicherte Produktion**: Stadtbanner, Stadtfenster und Produktionsfenster zeigen die
  Produktion, die eine Stadt in Reserve hält (Überschuss eines fertigen Elements oder eine Runde
  ohne Produktion; das Spiel bewahrt höchstens eine Runde auf) und die in der nächsten Runde zu
  ihrer Produktion hinzukommt. Im Banner einer Stadt ohne Produktion erscheint sie als **+N**.
- **BGE Check**: In einer Mehrspieler-Lobby hat der Host neben dem MOD CHECK von MPH einen Knopf
  **BGE CHECK**, der jeden Spieler nach seiner BetterGameExperience-Version fragt und das Ergebnis
  in den Chat schreibt: gleiche Version, andere Version, installiert aber deaktiviert oder keine
  Antwort.
- **Ankunftsmeldung**: Beim Öffnen einer Lobby mit aktiviertem BetterGameExperience meldet der
  Chat dies, mit Ihrer genauen Version.

### Korrekturen der Oberfläche
- **Stadtfenster nach dem Entfernen der Produktion**: Das Fenster unten rechts wird aktualisiert,
  wenn das einzige Element der Warteschlange entfernt wird, statt es weiter anzuzeigen.
- **Rechtsklick auf die aktuelle Produktion**: Er entfernt sie jetzt auch im Reiter Produktion,
  nicht nur bei geöffneter Warteschlange.
- **MPH-Lobby**: Außerhalb eines Drafts rückt die Spielerliste in den leeren Platz auf, den MPH
  für seinen Draft freihält, und der einzelne « . » darüber in einem geladenen Spiel verschwindet.

### Einstellungen
- Die drei Neuerungen haben je eine Einstellung im Reiter BetterGameExperience, standardmäßig aktiv.

## BetterGameExperience 0.8.0

Im Mehrspielermodus müssen alle menschlichen Spieler 0.8.0 haben. Die Korrektur für Abkommen am
Rundenende schützt nur die Spieler, die sie haben: Ein einziger Spieler, der noch 0.7.x hat, kann die
hängende Runde für alle zurückbringen. In der Lobby sieht der Host in Rot, wer eine andere Version
hat, und eine Lobby, die 0.7.x und 0.8.0 mischt, schaltet die Korrekturen **[alle Spieler]** ab.

### Mehrspielerkorrekturen [alle Spieler]
- **Runde in Endlosschleife nach einem Abkommen zwischen Menschen**: Ein Abkommen, das genau zum
  Ende deiner Runde vorgeschlagen oder angenommen wurde, konnte nach dem Rundenende beim Host
  ankommen. Die nächste Runde startete dann bei manchen Spielern immer wieder neu, man musste zurück
  in die Lobby, und das Spiel desynchronisierte oft. Ein solches Abkommen wird nicht mehr gesendet,
  sobald dein Timer abgelaufen ist oder kein Mensch mehr seine Runde aktiv hat: Eine Zeile in deinem
  Chat sagt es dir. Abkommen mit KIs bleiben unverändert.

### Neu
- Schaltfläche **Fehler melden (BGE)** im Esc-Menü, unter Optionen. Beschreibe das Problem: BGE
  sendet einen Spielstand, die letzten Autosaves und die Spielprotokolle (ohne deinen
  Windows-Benutzernamen), und eine Zeile in deinem Chat nennt die Nummer des Berichts. Die anderen
  Spieler sehen im Chat, dass ein Bericht gesendet wird. Schlägt das Senden fehl, wird das Zip auf
  deinen Desktop kopiert.

### Korrekturen der Oberfläche
- **Wirkung der Politikkarten**: Rationalismus, Große Oper, Simultaneum und Freie Marktwirtschaft
  zählen jetzt auch die festen Erträge, die den verstärkten Gebäuden hinzugefügt werden
  (Gouverneure…), nicht nur deren Grundertrag. Boni von Stadtstaaten auf diese Gebäude bleiben wie
  im Spiel außen vor, da das Spiel sie nicht vervielfacht.
- **Auswahl der Handelswege**: Bei einem Inlandshandelsweg wird jetzt das Gold angezeigt, das
  Poundmakers Fähigkeit der Zielstadt gibt (+1 pro Lager und pro Weide), mit einer Tooltip-Zeile, die
  die Stadt nennt.
- **Obere Leiste**: Die Bevölkerungsschaltfläche zeigt auch die durchschnittliche netto
  Annehmlichkeit pro Stadt (grün ab +3, rot unter 0).
- **Hauptmenü**: Die Version von BetterGameExperience wird unten links angezeigt, nach der des
  Spiels und der von MPH (`… - MPH / … - BGE / 0.8.0`).

### Einstellungen
- Spieloption **„No First/Last moves (BGE)“**: jetzt standardmäßig aktiviert. Der Host kann sie
  weiterhin abwählen.

## BetterGameExperience 0.7.0

Im Mehrspielermodus brauchen die Korrekturen **[alle Spieler]** alle menschlichen Spieler auf
0.7.0: Eine Lobby, die 0.6.x und 0.7.0 mischt, schaltet sie ab.

### Spielkorrekturen [alle Spieler]
- **Gestapelte Einheiten**: Zwei Militäreinheiten derselben Klasse können auf demselben Feld landen (in
  einer besetzten Stadt gekaufte Einheit, unterbrochene Bewegung, Teleport…). Die überzählige
  Einheit zählt nicht mehr für Flanken- und Unterstützungsboni: Zwei gestapelte Einheiten neben dem
  Ziel geben +2, nicht +4. Die Stadtverteidigung bleibt unverändert.

### Weltkongress
- **Entfernt**: die Korrekturen aus 0.6.x, die das Rundenende während der Abstimmung verhinderten (kein
  automatisches Rundenende, kein erzwungenes Rundenende bei leerer Resolutionsliste, 240 s auf dem
  Timer von MPH). Timer und Rundenende des Spiels und von MPH gelten während der Abstimmung wieder.
  Die Schaltfläche **Zurück zur Abstimmung**, der Begrüßungsbildschirm einmal pro Sitzung und die
  Neuberechnung der Luxusgüter bleiben.

### Korrekturen der Oberfläche
- **Stadtstaaten-Bildschirm**: Platzierte, aber noch nicht bestätigte Gesandte gehen nicht mehr
  verloren, wenn sich der Bildschirm aktualisiert (Gesandter eines anderen Spielers, Quest, Krieg
  oder Frieden). Sie werden nur entfernt, wenn der Stadtstaat zerstört wird oder Ihr Team ihm den
  Krieg erklärt.
- **Handelsbildschirm**: Die Schaltfläche für verfügbares Gold funktioniert wieder. Sie konnte nach
  einem früheren Handel nicht mehr anklickbar sein (Spielfehler).
- **Ganze Karte von Anfang an** (standardmäßig an): Minikarte und Kamera decken ab der ersten Runde
  die ganze Karte ab, Kartenmarkierungen an den Kartenrändern sind nicht mehr nötig. Unerforschte
  Gebiete bleiben verborgen. Rein lokal.
- **Bewegungsbereich einer Einheit auf einem feindlichen Bezirk**: keine Ruckler mehr beim Auswählen
  (unfertiges feindliches Lager), Kontrollzonen (rote Felder) werden
  angezeigt, und Wasserfelder, die die Einheit noch nicht betreten kann (Küste vor dem Einschiffen,
  Ozean vor Kartografie), werden nicht mehr angeboten.

### Einstellungen
- Schaltfläche **BGE zurücksetzen** im Reiter BetterGameExperience: setzt alle Einstellungen auf
  ihren Standardwert zurück.
- Neue erweiterte Diagnose-Einstellung, standardmäßig aus: **Kosten des Bewegungsbereichs messen (in Zeit)**.

## BetterGameExperience 0.6.1

Enthält alles aus der Vorabversion 0.6.0. Im Mehrspielermodus brauchen die Korrekturen **[alle
Spieler]** alle menschlichen Spieler auf 0.6.0 oder 0.6.1: Eine Lobby, die 0.5.0 und 0.6.x mischt,
schaltet sie ab.

### Installation und Updates
- Der Installer zeigt nach der Installation die Steam-Startoption nicht mehr an: Er meldet, dass
  alles bereit ist, und Sie können ihn schließen. Der Update-Hinweis im Spiel genügt, um aktuell zu
  bleiben. Eine bereits eingetragene Startoption funktioniert weiter.
- Der Installer heißt jetzt `BetterGameExperience_V<version>.exe`. Beim Start löscht er ältere
  Installer aus demselben Ordner (meist Downloads).

### Mehrspieler
- **„No First/Last moves (BGE)“** **[alle Spieler]**: Sie bleiben auf dem Rundenwechsel-Bildschirm,
  die Schaltfläche „Runde beenden“ eingeschlossen, bis der Host die Runde startet, statt Ihre Runde
  offen zu sehen und Ihre Befehle abgelehnt zu bekommen.

### Weltkongress
- *(experimentell)* **[alle Spieler]** Während Sie auf die anderen Spieler warten, öffnet die
  Schaltfläche **Zurück zur Abstimmung** das Abstimmungsfenster erneut, mit Ihren Stimmen und der
  richtigen Menge Diplomatischer Gunst.
- *(experimentell)* Die Runde endet während der Abstimmung nicht mehr gegen Ihren Willen: kein
  automatisches Rundenende, kein erzwungenes Rundenende bei leerer Resolutionsliste, und der Timer
  von MPH gibt die vorgesehenen 240 s für die Abstimmung.
- *(experimentell)* **[alle Spieler]** Die von einem Verbot betroffenen Luxusressourcen werden in
  jeder Sitzung neu berechnet.
- Der Begrüßungsbildschirm des Weltkongresses und sein Ton erscheinen nur noch einmal pro Sitzung,
  statt jedes Mal zurückzukehren, wenn ein anderer Spieler zur Abstimmung zurückkehrt.

### Korrekturen der Oberfläche
- Mit Better Spectator Mod zeigt die obere Leiste wieder die strategischen Ressourcen Ihrer
  Teamkameraden und die Ressourcen, die ein Teamkamerad freischaltet.

## BetterGameExperience 0.5.0: erste öffentliche Version

BetterGameExperience (BGE) behebt Fehler in Civilization VI: Gathering Storm und beschleunigt es, mit zwei DLLs
neben den Spieldateien. **Keine Spieldatei wird ersetzt**: Weder die Dateiprüfung von Steam noch
Spiel-Updates entfernen es. Funktioniert mit DirectX 11 und DirectX 12, mit BBG, Better Spectator Mod
und MPH.

Alle Änderungen sind unten nach Kategorien aufgeführt. Die mit **[alle Spieler]** markierten ändern
die Spielregeln: Sie werden nur aktiv, wenn alle menschlichen Spieler in der Lobby dieselbe Version
von BetterGameExperience haben. Alles andere betrifft nur Ihren eigenen Rechner. Man kann problemlos mit
Spielern ohne den Mod spielen.

### Installation und Updates
- Installationsprogramm `BetterGameExperience.exe`: Installieren, Aktualisieren, Reparieren und
  Deinstallieren. Es findet das Spiel in Ihren Steam-Bibliotheken und prüft das signierte Manifest
  und den SHA-256 jeder Datei, bevor es etwas kopiert.
- Zwei Dateien kommen neben die Spieldateien: `winmm.dll` und `BGD_GameCore_XP2.dll`. Ältere
  Installationen (0.3.0 und früher) werden automatisch bereinigt. Eine `winmm.dll` eines anderen Mods
  wird nie überschrieben.
- Update-Hinweis im Hauptmenü, auch im Menü von MPH. **Aktualisieren** schließt das Spiel, installiert
  die neue Version und bietet an, das Spiel neu zu starten.
- Kästchen „Vorabversion“ im Installationsprogramm, um kommende Versionen zu testen, mit Rückkehr zur
  stabilen Version per Klick.
- Optionale Steam-Startoption, die bei jedem Spielstart nach Updates sucht.
- Installationsprogramm in 11 Sprachen, denen von Better Balanced Game. Es folgt der Sprache von
  Windows, und unten im Fenster lässt sich eine andere wählen.

### Einstellungen
- Neuer Reiter **BetterGameExperience** in den Spieloptionen: ein Hauptschalter, der alles abschaltet, ohne
  zu deinstallieren, eine Einstellung pro Funktion und standardmäßig ausgeblendete erweiterte
  Optionen. Er weist darauf hin, wenn eine Änderung einen Neustart erfordert.
- Danksagungen an die Autoren der integrierten Oberflächen-Mods.
- Alle Texte im Spiel gibt es in den 11 Sprachen von Better Balanced Game: Englisch, Französisch,
  Deutsch, Spanisch, Italienisch, Japanisch, Koreanisch, Polnisch, brasilianisches Portugiesisch,
  Russisch und vereinfachtes Chinesisch.

### Mehrspieler
- **Prüfung in der Lobby**: Jeder Rechner meldet seine Version von BetterGameExperience, und der Host
  entscheidet für alle. Chatnachrichten mit dem Präfix `BGE -` zeigen, wem der Mod fehlt oder wer eine
  andere Version hat. Wiedereintritte werden berücksichtigt. Tritt jemand ohne den Mod bei, während
  Korrekturen **[alle Spieler]** aktiv sind, wird der Host gewarnt.
- `BGE -`-Chatnachrichten werden auf Englisch gesendet, damit Spieler ohne den Mod sie lesen können.
  Spieler mit dem Mod sehen sie in der Sprache ihres Spiels.
- **Spieloption „No First/Last moves (BGE)“** **[alle Spieler]**:
  - Alle Spieler beginnen ihre Runde gemeinsam. Befehle werden zurückgehalten und der Timer bleibt
    stehen, bis alle fertig geladen haben (höchstens 60 s).
  - Die Runde endet für alle, wenn der Timer des Hosts abläuft.
  - Die alte Rundensynchronisation von MPH wird abgeschaltet.
- Geladene Partien in der Lobby von MPH: Stadtstaaten verursachen kein „Ungültige Spieleroptionen“
  und keine ausgegraute Bereit-Schaltfläche mehr.

### Spielkorrekturen [alle Spieler]
- Bewirtschaftete Felder: Eine Stadt bewirtschaftet kein 2/1-Feld mehr, während ein 2/2-Feld frei ist
  (Fokus Nahrung und Produktion).
- Zuschauer von Better Spectator Mod zählen nicht mehr beim Tourismus. Sie können im Weltkongress
  nicht abstimmen und nicht Ziel einer Resolution sein. Den Kongressbildschirm können sie weiterhin
  ansehen.
- Marineeinheiten können sich von jedem Marinebezirk aus (Hafen, Wasserpark, Kanal …) in eine Stadt
  teleportieren, nicht mehr nur vom Stadtzentrum aus, so wie sie sich schon von einer Stadt in jeden
  Marinebezirk teleportieren konnten (kein Admiral mehr, der auf einem See aus einem Feld
  feststeckt).

### Korrekturen der Oberfläche
- Angriffsschaltflächen (Stadt, Lager, Atomschlag aus einem Silo) werden über den Einheitenflaggen
  gezeichnet, wo man sie anklicken kann.
- Ein Fernangriff aus einer nicht ausgewählten Stadt braucht einen Klick statt zwei.
- Im Zielmodus für den Fernangriff einer Stadt oder eines Bezirks feuert ein Klick auf die Flagge
  einer feindlichen Einheit auf sie.
- Die Kamera zentriert sich beim Klick auf eine Angriffsschaltfläche nicht mehr (lässt sich wieder
  einschalten).
- Der Produktions-Rundenzähler berücksichtigt jetzt die Regierungsboni: Faschismus, Autokratie und
  Handelsrepublik.
- Ein Weltwunder, das nach zwei Tauschvorgängen aus der Produktionswarteschlange fällt, wird wieder
  eingereiht.
- Die laufende Produktion wird beim Wechsel zu einer anderen Stadt richtig aktualisiert.
- Die Reichserträge der oberen Leiste aktualisieren sich nach einer Gouverneursaktion oder einer
  Großen Persönlichkeit.
- *(experimentell)* Tastenkürzel funktionieren weiter, nachdem ein Diplomatiebildschirm das Esc-Menü
  geschlossen hat oder nach einem doppelten Kongress-Popup.
- Esc schließt den Gesandten-Bildschirm jetzt richtig: Ausstehende Gesandte blockieren das Rundenende
  nicht mehr.
- *(experimentell)* Die Siedler-Linse wird nach einer Stadtgründung neu gezeichnet, und zu nah an
  einer Stadt liegende Felder werden als verboten angezeigt.
- Enter folgt Ihrer Tastenbelegung für „Nächste Aktion“: Sie können die Taste dort also entfernen
  und einer anderen Belegung zuweisen. Umschalt+Enter erzwingt weiterhin das Rundenende.
- Der Bewegungsbereich wird für eine Einheit auf einem feindlichen verteidigungsfähigen Bezirk
  angezeigt.
- *(experimentell)* Kartenmarkierungen werden nicht mehr neu aufgebaut, wenn sich nichts geändert
  hat: weniger Ruckler im Mehrspielermodus.

### Integrierte Oberflächen-Mods
Jeder lässt sich im Reiter BetterGameExperience abschalten. Haben Sie auch den Workshop-Mod, ersetzt ihn
die in BGE integrierte Version.
- **Ladungen der Handwerker** auf den Einheitenflaggen (nach Better Builder Charges Tracking, von
  wltk).
- **Verbleibende Runden der Handelswege** auf den Flaggen Ihrer Händler (neu).
- Schaltfläche **Benachrichtigungen löschen** (nach NotificationClear, von 336tarot).
- **Erweiterte obere Leiste**: Nahrung, Produktion, Bevölkerung und Luxusgüter, mit Details pro Stadt
  (nach TopPanel Extension Pro, von 号码菌 / Azukii).
- **Wirkung der Politikkarten**: Jede Karte zeigt ihren tatsächlichen Gewinn für Ihr Reich (nach
  Extended Policy Cards, von Aristos). Better Report Screen wird nicht mehr benötigt. Korrigierte
  Werte für:
  - Raj, Medina-Viertel, Mönchtum und Ziviles Prestige;
  - Rekrutierung und Massenaushebung;
  - die Karten Prätorium und Kolonialsteuern;
  - Punkte für Große Persönlichkeiten, jetzt als [Symbol]+x angezeigt.
- **Verbesserter Berichtsbildschirm** mit den Reitern Abkommen, Einheiten, Politiken, Stadtstaaten und
  Städte (nach Better Report Screen, von Infixo), mit denselben Korrekturen.

### Leistung und Komfort
- Schnellere Rundenberechnung:
  - Während einer Kaskade von Kriegserklärungen werden Stadt- und Diplomatiedaten nur einmal
    aktualisiert. Die gemessene Runde war 2,4 s schneller (−82 % bei der Kaskade): deutlich weniger
    Ruckler, wenn Stadtstaaten in einem Krieg die Seite wechseln.
  - Zwei Caches beschleunigen die Runde der KI: baubare Modernisierungen und Feld-Erträge.
  - Die Ergebnisse sind identisch mit dem Grundspiel: sicher mit Spielern ohne den Mod.
- Die Animation von Atomschlägen läuft mit Schnellkampf 20-mal schneller: etwa 0,6 s statt 11 s. Nur
  Anzeige.
- Der Bildratenbegrenzer bietet 45, 75, 90 und 100 FPS.

### Bekannte Einschränkungen
- Der Mod zielt auf die aktuelle offizielle Version des Spiels. Nach einem Spiel-Update schalten
  sich die betroffenen Funktionen von selbst ab, bis BetterGameExperience aktualisiert ist.
- Ein Spieler, der mitten in der Partie ohne den Mod beitritt, während Korrekturen **[alle Spieler]**
  aktiv sind, wird desynchronisiert. Der Host wird gewarnt.
- Mit *(experimentell)* markierte Korrekturen wurden noch nicht in einer echten Partie getestet,
  oder ihre Wirkung ist noch nicht vollständig gemessen.
- Die Übersetzungen außer Englisch und Französisch wurden noch nicht von Muttersprachlern geprüft.
  Korrekturen sind willkommen.
