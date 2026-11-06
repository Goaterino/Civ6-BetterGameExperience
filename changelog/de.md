# Änderungsprotokoll

[English](../CHANGELOG.md) · weitere Sprachen: siehe das [englische Protokoll](../CHANGELOG.md).

## BetterGameDLLs 0.5.0: erste öffentliche Version

BetterGameDLLs behebt Fehler in Civilization VI: Gathering Storm und beschleunigt es, mit zwei DLLs
neben den Spieldateien. **Keine Spieldatei wird ersetzt**: Weder die Dateiprüfung von Steam noch
Spiel-Updates entfernen es. Funktioniert mit DirectX 11 und DirectX 12, mit BBG, Better Spectator Mod
und MPH.

Alle Änderungen sind unten nach Kategorien aufgeführt. Die mit **[alle Spieler]** markierten ändern
die Spielregeln: Sie werden nur aktiv, wenn alle menschlichen Spieler in der Lobby dieselbe Version
von BetterGameDLLs haben. Alles andere betrifft nur Ihren eigenen Rechner. Man kann problemlos mit
Spielern ohne den Mod spielen.

### Installation und Updates
- Installationsprogramm `BetterGameDLLs.exe`: Installieren, Aktualisieren, Reparieren und
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
- Neuer Reiter **BetterGameDLLs** in den Spieloptionen: ein Hauptschalter, der alles abschaltet, ohne
  zu deinstallieren, eine Einstellung pro Funktion und standardmäßig ausgeblendete erweiterte
  Optionen. Er weist darauf hin, wenn eine Änderung einen Neustart erfordert.
- Danksagungen an die Autoren der integrierten Oberflächen-Mods.
- Alle Texte im Spiel gibt es in den 11 Sprachen von Better Balanced Game: Englisch, Französisch,
  Deutsch, Spanisch, Italienisch, Japanisch, Koreanisch, Polnisch, brasilianisches Portugiesisch,
  Russisch und vereinfachtes Chinesisch.

### Mehrspieler
- **Prüfung in der Lobby**: Jeder Rechner meldet seine Version von BetterGameDLLs, und der Host
  entscheidet für alle. Chatnachrichten mit dem Präfix `BGD -` zeigen, wem der Mod fehlt oder wer eine
  andere Version hat. Wiedereintritte werden berücksichtigt. Tritt jemand ohne den Mod bei, während
  Korrekturen **[alle Spieler]** aktiv sind, wird der Host gewarnt.
- `BGD -`-Chatnachrichten werden auf Englisch gesendet, damit Spieler ohne den Mod sie lesen können.
  Spieler mit dem Mod sehen sie in der Sprache ihres Spiels.
- **Spieloption „No First/Last moves (BGD)“** **[alle Spieler]**:
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
Jeder lässt sich im Reiter BetterGameDLLs abschalten. Haben Sie auch den Workshop-Mod, ersetzt ihn
die in BGD integrierte Version.
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
  sich die betroffenen Funktionen von selbst ab, bis BetterGameDLLs aktualisiert ist.
- Ein Spieler, der mitten in der Partie ohne den Mod beitritt, während Korrekturen **[alle Spieler]**
  aktiv sind, wird desynchronisiert. Der Host wird gewarnt.
- Mit *(experimentell)* markierte Korrekturen wurden noch nicht in einer echten Partie getestet,
  oder ihre Wirkung ist noch nicht vollständig gemessen.
- Die Übersetzungen außer Englisch und Französisch wurden noch nicht von Muttersprachlern geprüft.
  Korrekturen sind willkommen.
