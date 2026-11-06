# Registro delle modifiche

[English](../CHANGELOG.md) · altre lingue: vedi il [registro in inglese](../CHANGELOG.md).

## BetterGameDLLs 0.5.0: prima versione pubblica

BetterGameDLLs corregge e velocizza Civilization VI: Gathering Storm con due DLL posate accanto ai
file del gioco. **Nessun file del gioco viene sostituito**: né la verifica dei file di Steam né gli
aggiornamenti del gioco lo rimuovono. Funziona con DirectX 11 e DirectX 12, con BBG, Better Spectator
Mod e MPH.

Tutte le modifiche sono elencate qui sotto, per categoria. Quelle segnate **[tutti i giocatori]**
cambiano le regole della partita: si attivano solo quando tutti i giocatori umani della sala hanno la
stessa versione di BetterGameDLLs. Tutto il resto riguarda solo il tuo computer. Si può giocare senza
rischi con chi non ha la mod.

### Installazione e aggiornamenti
- Programma di installazione `BetterGameDLLs.exe`: installa, aggiorna, ripara e disinstalla. Trova il
  gioco nelle tue librerie di Steam e verifica il manifesto firmato e lo SHA-256 di ogni file prima di
  copiare qualsiasi cosa.
- Due file vengono aggiunti accanto a quelli del gioco: `winmm.dll` e `BGD_GameCore_XP2.dll`. Le
  installazioni precedenti (0.3.0 e prima) vengono ripulite automaticamente. Un `winmm.dll` di
  un'altra mod non viene mai sovrascritto.
- Avviso di aggiornamento nel menu principale, anche in quello di MPH. **Aggiorna** chiude il gioco,
  installa la nuova versione e propone di riavviarlo.
- Casella «Versione di prova» nel programma di installazione per provare le prossime versioni, con
  ritorno alla versione stabile in un clic.
- Opzione di avvio di Steam facoltativa, che cerca aggiornamenti a ogni avvio del gioco.
- Programma di installazione in 11 lingue, quelle di Better Balanced Game. Segue la lingua di Windows,
  e se ne può scegliere un'altra in fondo alla finestra.

### Impostazioni
- Nuova scheda **BetterGameDLLs** nelle opzioni del gioco: un interruttore generale che disattiva tutto
  senza disinstallare, un'impostazione per funzione e opzioni avanzate nascoste per impostazione
  predefinita. Avvisa quando una modifica richiede un riavvio.
- Riconoscimenti per gli autori delle mod dell'interfaccia integrate.
- Tutti i testi del gioco sono disponibili nelle 11 lingue di Better Balanced Game: inglese, francese,
  tedesco, spagnolo, italiano, giapponese, coreano, polacco, portoghese brasiliano, russo e cinese
  semplificato.

### Multigiocatore
- **Controllo nella sala**: ogni computer comunica la sua versione di BetterGameDLLs, e l'host decide
  per tutti. I messaggi di chat con il prefisso `BGD -` indicano a chi manca la mod o chi ha un'altra
  versione. Le riconnessioni sono gestite. Se qualcuno entra senza la mod mentre le correzioni
  **[tutti i giocatori]** sono attive, l'host viene avvisato.
- I messaggi di chat `BGD -` vengono inviati in inglese, così chi non ha la mod può leggerli. Chi ha la
  mod li vede nella lingua del proprio gioco.
- **Opzione di partita «No First/Last moves (BGD)»** **[tutti i giocatori]**:
  - Tutti i giocatori iniziano il turno insieme. Gli ordini vengono trattenuti, e il timer resta
    fermo, finché tutti non hanno finito di caricare (al massimo 60 s).
  - Il turno finisce per tutti quando scade il timer dell'host.
  - La sincronizzazione del turno di MPH, che non sincronizzava nulla, viene disattivata.
- Partite caricate nella sala di MPH: le città-stato non causano più «Opzioni giocatore non valide»
  né il pulsante Pronto disattivato.

### Correzioni di gioco [tutti i giocatori]
- Caselle lavorate: una città non lavora più una casella 2/1 mentre una 2/2 è libera (priorità cibo e
  produzione).
- Gli spettatori di Better Spectator Mod non contano più nel turismo. Non possono votare al Congresso
  Mondiale né essere il bersaglio di una risoluzione. Possono comunque consultare la schermata del
  Congresso.
- Le unità navali possono teletrasportarsi in una città da qualsiasi distretto (porto, parco
  acquatico, canale…), non più solo dal centro cittadino.

### Correzioni dell'interfaccia
- I pulsanti di attacco (città, accampamento, attacco nucleare da un silo) vengono disegnati sopra le
  bandiere delle unità, dove si possono cliccare.
- L'attacco a distanza da una città non selezionata richiede un clic, non due.
- In modalità mira, cliccare sulla bandiera di un'unità nemica la attacca.
- La visuale non si centra più al clic su un pulsante di attacco (si può riattivare).
- Il contatore dei turni di produzione ora include i bonus di governo: fascismo, autocrazia e
  repubblica mercantile.
- Una meraviglia tolta dalla coda di produzione dopo due scambi viene rimessa.
- La produzione in corso resta visibile quando si passa a un'altra città.
- Le rese dell'impero nella barra superiore si aggiornano dopo un'azione di un governatore o un grande
  personaggio.
- *(sperimentale)* Le scorciatoie da tastiera non smettono più di funzionare quando una schermata di
  diplomazia chiude il menu Esc, o dopo un doppio popup del Congresso.
- Esc ora chiude correttamente la schermata degli inviati: gli inviati in sospeso non bloccano più la
  fine del turno.
- *(sperimentale)* La lente del colono viene ridisegnata dopo la fondazione di una città, e le caselle
  troppo vicine a una città appaiono come vietate.
- Invio segue il tuo tasto per «Azione successiva». Maiusc+Invio forza sempre la fine del turno.
- L'area di movimento viene mostrata per un'unità su un distretto difendibile nemico.
- I segnalini della mappa non vengono più ricostruiti quando nulla è cambiato: meno rallentamenti in
  multigiocatore.

### Mod dell'interfaccia integrate
Ognuna si può disattivare nella scheda BetterGameDLLs. Se hai anche la mod del Workshop, la versione
integrata la sostituisce.
- **Cariche dei costruttori** sulle bandiere delle unità (da Better Builder Charges Tracking, di wltk).
- **Turni rimanenti delle rotte commerciali** sulle bandiere dei tuoi commercianti (nuovo).
- Pulsante **Cancella notifiche** (da NotificationClear, di 336tarot).
- **Barra superiore estesa**: cibo, produzione, popolazione e lussi, con il dettaglio per città (da
  TopPanel Extension Pro, di 号码菌 / Azukii).
- **Effetto delle carte politica**: ogni carta mostra il suo guadagno reale per il tuo impero (da
  Extended Policy Cards, di Aristos). Better Report Screen non serve più. Valori corretti per:
  - Raj, Quartiere della Medina, Monachesimo e Prestigio civile;
  - Coscrizione e Levée en Masse;
  - le carte Pretorio e Tasse coloniali;
  - i punti dei grandi personaggi, ora mostrati come [icona]+x.
- **Schermata dei rapporti migliorata** con le schede Accordi, Unità, Politiche, Città-stato e Città
  (da Better Report Screen, di Infixo), con le stesse correzioni.

### Prestazioni e comodità
- Cambio di turno più rapido:
  - Durante una cascata di dichiarazioni di guerra, i dati delle città e della diplomazia vengono
    aggiornati una sola volta. Il turno misurato è stato più rapido di 2,4 s (−82 % sulla cascata).
  - Due cache accelerano il turno dell'IA: miglioramenti costruibili e rese delle caselle.
  - I risultati sono identici a quelli del gioco base: nessun rischio con chi non ha la mod.
- L'animazione degli attacchi nucleari è 20 volte più rapida con il combattimento rapido: circa 0,6 s
  invece di 11 s. Solo visualizzazione.
- Il limitatore di fotogrammi offre 45, 75, 90 e 100 FPS.

### Limiti noti
- La mod è fatta per la versione attuale del gioco. Dopo un aggiornamento del gioco, le funzioni
  interessate si disattivano da sole finché BetterGameDLLs non viene aggiornato.
- Un giocatore che entra a partita in corso senza la mod, mentre le correzioni **[tutti i
  giocatori]** sono attive, si desincronizza. L'host viene avvisato.
- Le correzioni segnate *(sperimentale)* non sono ancora state provate in una partita reale.
- Le traduzioni diverse da inglese e francese non sono ancora state riviste da madrelingua. Le
  correzioni sono benvenute.
