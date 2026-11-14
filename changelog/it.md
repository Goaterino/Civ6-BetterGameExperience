# Registro delle modifiche

[English](../CHANGELOG.md) · altre lingue: vedi il [registro in inglese](../CHANGELOG.md).

## BetterGameExperience 0.11.0

In multigiocatore, tutti i giocatori umani devono avere la 0.11.0 per le correzioni **[tutti i
giocatori]**: una sala d'attesa che mescola 0.10.0 e 0.11.0 le disattiva.

### Correzioni multigiocatore [tutti i giocatori]
- **Bonus di produzione delle unità che si consumano**: rimuovere un bonus percentuale alla
  produzione delle unità (il +30 % ai costruttori di Ilkum, o il bonus di una città a un tipo di
  unità) toglieva un po' più di quanto avesse aggiunto, circa lo 0,4 % ogni volta, e la perdita
  cresceva a ogni scambio della carta. Rimuovere un bonus ora toglie esattamente ciò che aveva
  aggiunto. I salvataggi conservano la perdita già accumulata. Contributo di Serianol.

## BetterGameExperience 0.10.0

In multigiocatore, tutti i giocatori umani devono avere la 0.10.0 per le correzioni **[tutti i
giocatori]** e i segnali di squadra: una sala d'attesa che mescola 0.9.x e 0.10.0 li disattiva.

### Correzioni multigiocatore [tutti i giocatori]
- **Giocatori bloccati su « non pronto »**: un'azione diplomatica (ambasciata, accordo) ricevuta
  proprio alla fine del turno poteva far iniziare all'host il turno successivo troppo presto. Alcuni
  giocatori lo iniziavano allora già segnati come finiti e non potevano tornare pronti. Il turno
  successivo ora attende il controllo del gioco, e l'azione viene comunque applicata.
- **Spettatori (Better Spectator Mod)**: la revisione degli spettatori della 0.9.0 è sospesa in
  attesa di test più approfonditi, perché le partite con uno spettatore erano meno stabili.
  BetterGameExperience non cambia più il livello di civiltà degli spettatori e non sostituisce più
  gli script di Better Spectator Mod, e l'impostazione « Better Spectator Mod per gli spettatori »
  scompare per ora. Gli spettatori contano di nuovo come civiltà principali per script, vittorie e
  punteggio, come con il solo Better Spectator Mod. Turismo e votazioni del Congresso Mondiale
  continuano a escluderli.
- **Mantenuto per gli spettatori**: cambiare il giocatore osservato continua a non desincronizzare la
  partita, e il turno dello spettatore continua a terminare da solo dopo una pausa di MPH. Entrambe le
  correzioni si applicano ora agli script originali di Better Spectator Mod.

### Novità
- **Segnali di squadra (BGE)**: segna una casella per la tua squadra con **X**, o tieni premuto **X**
  per una ruota di avvisi (pericolo, sto arrivando, aiuto, nemico sparito); **Alt+1–4** li inviano
  direttamente. I compagni vedono e sentono il segnale con il nome di chi lo invia, gli spettatori
  vedono entrambe le squadre. Disattivato per impostazione predefinita: l'host spunta **Segnali di
  squadra (BGE)** nella configurazione della partita. Scorciatoie nelle opzioni dei tasti del gioco,
  volume nelle opzioni audio; `/mute nome` blocca chat e segnali di un giocatore. Contributo di
  mariusbw.

### Multigiocatore
- **Giocatori inoltrati dall'host**: due giocatori la cui connessione diretta è fallita all'ingresso
  passano per l'host per il resto della partita, e vengono entrambi disconnessi se l'host esce.
  Durante l'inizio del turno simultaneo, BetterGameExperience ora prova a ricollegarli direttamente
  (al massimo tre tentativi) e lo annuncia in chat. Contributo di yodha.
- **Ping bloccati a 0 ms**: il ping di un giocatore nell'elenco dei giocatori non resta più fermo a
  0 ms dopo l'ingresso; viene di nuovo misurato ogni 5 secondi. Non serve più uscire e rientrare per
  « sistemarlo », cosa che spesso causava una desincronizzazione. Contributo di yodha.
- **Messaggi di rete danneggiati**: la protezione aggiunta nella 0.9.0 funziona anche con la
  versione DirectX 11 del gioco. Contributo di yodha.
- **Giocatori senza BetterGameExperience**: l'avviso in partita su un giocatore entrato senza
  BetterGameExperience non si interrompe più per un errore di script. Contributo di yodha.

### Installazione e aggiornamenti
- Il programma di installazione scarica e riavvia il gioco senza avviare programmi ausiliari
  nascosti, e contiene informazioni di versione: gli antivirus dovrebbero segnalarlo meno spesso.

## BetterGameExperience 0.9.1

Compatibile con la 0.9.0 in multigiocatore: giocatori con 0.9.0 e 0.9.1 giocano insieme con tutte le
correzioni.

### Correzioni dell'interfaccia
- **Schermata dei rapporti**: le schede Rendite e Città funzionano di nuovo. Il calcolo dell'effetto
  delle carte politica, caricato nella stessa schermata, sostituiva alcuni dei suoi dati e le
  rompeva. Le schede Politiche e Città-stato non falliscono più su una riga con punti personaggio
  illustre, e ordinare una colonna della scheda Città non fallisce più quando la scheda Rendite non
  è stata aperta prima.
- **Distretti scontati**: lo sconto mostrato è ora quello che il gioco applica. Il gioco aggiorna il
  costo di un distretto solo in certi momenti (nuova tecnologia o nuovo progresso civico, nuova
  città, distretto dello stesso tipo posizionato o completato), quindi l'elenco poteva mostrare uno
  sconto che il gioco concedeva solo qualche turno dopo.

## BetterGameExperience 0.9.0

Comprende tutto il contenuto della pre-release 0.8.1. In multigiocatore, tutti i giocatori umani
devono avere la 0.9.0 per le correzioni **[tutti i giocatori]**: una sala d'attesa che mescola 0.8.x
e 0.9.0 le disattiva.

### Correzioni del multigiocatore [tutti i giocatori]
- **Spettatori (Better Spectator Mod)**: non contano più come civiltà maggiori ed escono dagli
  elenchi dei giocatori degli script, delle vittorie e del punteggio. BetterGameExperience include
  le proprie versioni degli script di Better Spectator Mod, fatte per questo. Contributo di
  banipouz.
- **Desincronizzazione causata da uno spettatore**: cambiare il giocatore osservato da uno
  spettatore non desincronizza più la partita (Better Spectator Mod lo scriveva nei dati della
  partita, solo sul computer dello spettatore).
- **Spettatori nella barra dei leader**: i giocatori non vedono più gli spettatori nella barra in
  alto a destra, dove apparivano come un'IA, con un ritratto « ? », il loro nome e le loro rese.
- **Turno dello spettatore dopo una pausa**: dopo una pausa di MPH, il turno dello spettatore torna
  a finire da solo, invece di aspettare che lo passi a mano.

### Multigiocatore
- **Messaggi di rete danneggiati**: un messaggio danneggiato, visto alla disconnessione di un
  giocatore, poteva bloccare il gioco fino all'esaurimento della memoria qualche minuto dopo. I
  valori impossibili di un tale messaggio ora vengono scartati e la partita continua; una riga in
  chat lo dice a tutti. Una configurazione di partita o di giocatore danneggiata non viene più
  applicata, invece di cancellare la configurazione. Protegge i giocatori che hanno
  BetterGameExperience. Contributo di yodha.
- **No First/Last moves con un host spettatore**: il turno ora parte appena i giocatori sono pronti,
  invece di trattenerli fino al tempo di sicurezza.

### Novità
- **Avanzamento di ricerca e civico**: nel pannello in alto a sinistra, accanto al nome della
  tecnologia e del civico in corso, **x/y** indica la scienza (la cultura) già investita e il costo
  totale, seguito da **(+z)** quando il gioco conserva scienza (cultura) in eccesso da una
  tecnologia (un civico) completata con più del suo costo. Un'impostazione permette di disattivarlo.
- **Disable Advanced UI indicators (BGE)**: nuova opzione di partita, subito sotto No First/Last
  moves, disattivata per impostazione predefinita. Se l'host la spunta, nessun giocatore vede
  l'avanzamento di ricerca e civico (**x/y (+z)**) né la produzione conservata dalle città,
  qualunque siano le sue impostazioni. Viene riletta a ogni partita: in una partita senza l'opzione
  ricompaiono, senza riavviare Civ VI.
- **Annullare la produzione dallo stendardo della città**: un clic destro sull'icona di produzione
  dello stendardo di una delle tue città rimuove l'elemento in corso, come nel pannello di
  produzione.
- **Distretti scontati**: nell'elenco di produzione, un distretto con costo ridotto (hai meno
  distretti di quel tipo della tua media) mostra lo sconto, per esempio **-40%**. Si aggiorna non
  appena viene posizionato un distretto, perché posizionarne uno può far perdere lo sconto. Anche i
  distretti che non contano nella media, come il Thành del Vietnam, sono indicati.
- **Produzione in riserva**: lo stendardo della città, il pannello della città e il pannello di
  produzione mostrano la produzione che la città tiene in riserva (eccedenza di un elemento
  completato, o un turno senza produrre; il gioco ne conserva al massimo un turno), aggiunta alla
  sua produzione del turno successivo. Sullo stendardo di una città senza produzione appare come
  **+N**.
- **BGE Check**: in una sala d'attesa multigiocatore, l'host ha un pulsante **BGE CHECK**, accanto
  al MOD CHECK di MPH, che chiede a ogni giocatore la sua versione di BetterGameExperience e scrive
  il risultato in chat: stessa versione, altra versione, installato ma disattivato o nessuna
  risposta.
- **Messaggio di arrivo**: all'apertura di una sala d'attesa con BetterGameExperience attivo, la
  chat lo annuncia, con la tua versione esatta.

### Correzioni dell'interfaccia
- **Pannello della città dopo aver rimosso la produzione**: il pannello in basso a destra si
  aggiorna quando si rimuove l'unico elemento della coda, invece di mostrarlo ancora.
- **Clic destro sulla produzione in corso**: ora la rimuove anche dalla scheda Produzione, non solo
  con la coda aperta.
- **Sala d'attesa di MPH**: fuori da un draft, l'elenco dei giocatori risale nello spazio vuoto che
  MPH riserva al suo draft, e il « . » mostrato sopra in una partita ricaricata scompare.
- **Schermata diplomatica**: dopo l'incontro con un giocatore la cui squadra mancava dall'elenco
  delle squadre del gioco, le icone dei leader fallivano a ogni turno, e aprire la schermata
  diplomatica poteva nascondere tutta l'interfaccia per sempre. Le icone funzionano di nuovo e, se
  la schermata diplomatica non si apre, l'interfaccia ritorna.
- **Imhotep**: il suo effetto mostra ora la quantità alla velocità della partita (87, o 175 per una
  meraviglia antica o classica, a velocità Online), nella schermata dei personaggi illustri e nella
  Civilopedia, invece di 175 in ogni caso.
- **Pausa di MPH**: il menu MULTIPLAYER HELPER e il voto di remap ora appaiono sopra la schermata di
  pausa di MPH, con pulsanti cliccabili.
- **Barra superiore**: i servizi medi per città sono arrotondati all'intero più vicino invece che
  per difetto.
- **Segnala un bug (BGE)**: il salvataggio è ora allegato alla segnalazione; partivano solo i
  salvataggi automatici.

### Impostazioni
- Le novità dell'interfaccia (annullare dallo stendardo, distretti scontati, produzione conservata,
  avanzamento di ricerca e civico) e gli script di Better Spectator Mod hanno ciascuno
  un'impostazione nella scheda BetterGameExperience, attiva per impostazione predefinita.

## BetterGameExperience 0.8.0

In multigiocatore, tutti i giocatori umani devono avere la 0.8.0. La correzione degli accordi a fine
turno protegge solo chi la ha: un solo giocatore rimasto alla 0.7.x può riportare il turno bloccato
per tutti. Nella lobby, l'host vede in rosso chi ha un'altra versione, e una lobby che mescola 0.7.x e
0.8.0 disattiva le correzioni **[tutti i giocatori]**.

### Correzioni del multigiocatore [tutti i giocatori]
- **Turno in loop dopo un accordo tra umani**: un accordo proposto o accettato proprio alla fine del
  tuo turno poteva arrivare all'host dopo la fine del turno. Il turno successivo ripartiva allora
  all'infinito per alcuni giocatori, bisognava tornare alla lobby e la partita spesso si
  desincronizzava. Un accordo del genere non viene più inviato una volta scaduto il tuo timer, né
  quando nessun umano ha più il turno attivo: una riga nella tua chat te lo segnala. Gli accordi con
  le IA non cambiano.

### Novità
- Pulsante **Segnala un bug (BGE)** nel menu Esc, sotto Opzioni. Descrivi il problema: BGE invia un
  salvataggio, gli ultimi salvataggi automatici e i registri del gioco (senza il tuo nome utente
  Windows), e una riga della tua chat dà il numero del rapporto. Gli altri giocatori vedono nella chat
  che un rapporto è in corso di invio. Se l'invio non riesce, lo zip viene copiato sul tuo Desktop.

### Correzioni dell'interfaccia
- **Effetto delle carte politica**: Razionalismo, Grand opéra, Simultaneum e Libero mercato contano
  anche le rese fisse aggiunte agli edifici che potenziano (governatori…), non solo la loro resa di
  base. I bonus delle città-stato su questi edifici restano esclusi, come nel gioco, che non li
  moltiplica.
- **Scelta delle rotte commerciali**: per una rotta interna, ora viene mostrato l'oro che l'abilità
  di Poundmaker dà alla città di destinazione (+1 per accampamento e per pascolo), con una riga del
  tooltip che nomina la città.
- **Barra superiore**: il pulsante della popolazione mostra anche la media dei servizi netti per città
  (in verde da +3, in rosso sotto 0).
- **Menu principale**: la versione di BetterGameExperience compare in basso a sinistra, dopo quella
  del gioco e quella di MPH (`… - MPH / … - BGE / 0.8.0`).

### Impostazioni
- Opzione di partita **«No First/Last moves (BGE)»**: ora attiva per impostazione predefinita. L'host
  può sempre disattivarla.

## BetterGameExperience 0.7.0

In multigiocatore, le correzioni **[tutti i giocatori]** richiedono che tutti i giocatori umani
abbiano la 0.7.0: una lobby che mescola 0.6.x e 0.7.0 le disattiva.

### Correzioni di gioco [tutti i giocatori]
- **Unità impilate**: due unità militari della stessa classe possono finire sulla stessa casella (unità
  acquistata in una città occupata, movimento interrotto, teletrasporto…). L'unità in eccesso non
  conta più nei bonus di fiancheggiamento e di supporto: due unità impilate accanto al bersaglio
  danno +2, non +4. La difesa delle città non cambia.

### Congresso Mondiale
- **Rimosse**: le correzioni della 0.6.x che impedivano la fine del turno durante il voto (niente fine
  turno automatica, niente fine turno forzata con una lista di risoluzioni vuota, 240 s sul timer di
  MPH). Il timer e la fine del turno del gioco e di MPH tornano ad applicarsi durante il voto. Il
  pulsante **Torna al voto**, la schermata di benvenuto una volta per sessione e il ricalcolo dei
  lussi restano.

### Correzioni dell'interfaccia
- **Schermata delle città-stato**: gli emissari piazzati ma non ancora confermati non vanno più persi
  quando la schermata si aggiorna (emissario di un altro giocatore, missione, guerra o pace).
  Vengono tolti solo se la città-stato viene distrutta o se la tua squadra le dichiara guerra.
- **Schermata degli accordi**: il pulsante dell'oro disponibile funziona di nuovo. Poteva restare non
  cliccabile dopo un accordo precedente (bug del gioco).
- **Mappa intera fin dall'inizio** (attiva di default): la minimappa e la telecamera coprono tutta
  la mappa dal primo turno, non servono più segnalini ai bordi della mappa. Le zone inesplorate
  restano nascoste. Puramente locale.
- **Area di movimento di un'unità su un distretto nemico**: niente più scatti quando la selezioni
  (accampamento nemico non finito), le zone di controllo (caselle rosse) vengono
  mostrate e le caselle d'acqua in cui l'unità non può ancora entrare (costa prima
  dell'imbarco, oceano prima della Cartografia) non vengono più proposte.

### Impostazioni
- Pulsante **Ripristina BGE** nella scheda BetterGameExperience: riporta tutte le impostazioni al
  loro valore predefinito.
- Nuova impostazione avanzata di diagnostica, disattivata di default: **Misura il costo dell'area di
  movimento (in tempo)**.

## BetterGameExperience 0.6.1

Include tutto della pre-release 0.6.0. In multigiocatore, le correzioni **[tutti i giocatori]**
richiedono che tutti i giocatori umani abbiano la 0.6.0 o la 0.6.1: una lobby che mescola 0.5.0 e
0.6.x le disattiva.

### Installazione e aggiornamenti
- Dopo l'installazione, l'installer non mostra più l'opzione di avvio di Steam: dice che è tutto
  pronto e puoi chiuderlo. L'avviso di aggiornamento del gioco basta per restare aggiornati.
  Un'opzione di avvio già aggiunta continua a funzionare.
- L'installer ora si chiama `BetterGameExperience_V<version>.exe`. All'avvio elimina gli installer
  più vecchi dalla stessa cartella (di solito Download).

### Multigiocatore
- **"No First/Last moves (BGE)"** **[tutti i giocatori]**: resti sulla schermata di passaggio del
  turno, pulsante di fine turno compreso, finché l'host non avvia il turno, invece di vedere il tuo
  turno aperto con gli ordini rifiutati.

### Congresso Mondiale
- *(sperimentale)* **[tutti i giocatori]** Mentre aspetti gli altri giocatori, un pulsante **Torna
  al voto** riapre la finestra di voto con i tuoi voti e la giusta quantità di Favore Diplomatico.
- *(sperimentale)* Il turno non termina più contro la tua volontà durante il voto: niente fine turno
  automatica, niente fine turno forzata con un elenco di risoluzioni vuoto, e il timer di MPH dà i
  240 s previsti per il voto.
- *(sperimentale)* **[tutti i giocatori]** I lussi colpiti da un divieto vengono ricalcolati a ogni
  sessione.
- La schermata di benvenuto del Congresso Mondiale e il suo suono compaiono una sola volta per
  sessione, invece di tornare ogni volta che un altro giocatore torna al voto.

### Correzioni dell'interfaccia
- Con Better Spectator Mod, la barra superiore mostra di nuovo le risorse strategiche dei tuoi
  compagni di squadra e le risorse che un compagno sblocca.

## BetterGameExperience 0.5.0: prima versione pubblica

BetterGameExperience (BGE) corregge e velocizza Civilization VI: Gathering Storm con due DLL posate accanto ai
file del gioco. **Nessun file del gioco viene sostituito**: né la verifica dei file di Steam né gli
aggiornamenti del gioco lo rimuovono. Funziona con DirectX 11 e DirectX 12, con BBG, Better Spectator
Mod e MPH.

Tutte le modifiche sono elencate qui sotto, per categoria. Quelle segnate **[tutti i giocatori]**
cambiano le regole della partita: si attivano solo quando tutti i giocatori umani della sala hanno la
stessa versione di BetterGameExperience. Tutto il resto riguarda solo il tuo computer. Si può giocare senza
rischi con chi non ha la mod.

### Installazione e aggiornamenti
- Programma di installazione `BetterGameExperience.exe`: installa, aggiorna, ripara e disinstalla. Trova il
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
- Nuova scheda **BetterGameExperience** nelle opzioni del gioco: un interruttore generale che disattiva tutto
  senza disinstallare, un'impostazione per funzione e opzioni avanzate nascoste per impostazione
  predefinita. Avvisa quando una modifica richiede un riavvio.
- Riconoscimenti per gli autori delle mod dell'interfaccia integrate.
- Tutti i testi del gioco sono disponibili nelle 11 lingue di Better Balanced Game: inglese, francese,
  tedesco, spagnolo, italiano, giapponese, coreano, polacco, portoghese brasiliano, russo e cinese
  semplificato.

### Multigiocatore
- **Controllo nella sala**: ogni computer comunica la sua versione di BetterGameExperience, e l'host decide
  per tutti. I messaggi di chat con il prefisso `BGE -` indicano a chi manca la mod o chi ha un'altra
  versione. Le riconnessioni sono gestite. Se qualcuno entra senza la mod mentre le correzioni
  **[tutti i giocatori]** sono attive, l'host viene avvisato.
- I messaggi di chat `BGE -` vengono inviati in inglese, così chi non ha la mod può leggerli. Chi ha la
  mod li vede nella lingua del proprio gioco.
- **Opzione di partita «No First/Last moves (BGE)»** **[tutti i giocatori]**:
  - Tutti i giocatori iniziano il turno insieme. Gli ordini vengono trattenuti, e il timer resta
    fermo, finché tutti non hanno finito di caricare (al massimo 60 s).
  - Il turno finisce per tutti quando scade il timer dell'host.
  - La vecchia sincronizzazione del turno di MPH viene disattivata.
- Partite caricate nella sala di MPH: le città-stato non causano più «Opzioni giocatore non valide»
  né il pulsante Pronto disattivato.

### Correzioni di gioco [tutti i giocatori]
- Caselle lavorate: una città non lavora più una casella 2/1 mentre una 2/2 è libera (priorità cibo e
  produzione).
- Gli spettatori di Better Spectator Mod non contano più nel turismo. Non possono votare al Congresso
  Mondiale né essere il bersaglio di una risoluzione. Possono comunque consultare la schermata del
  Congresso.
- Le unità navali possono teletrasportarsi in una città da qualsiasi distretto navale (porto, parco
  acquatico, canale…), non più solo dal centro cittadino, così come potevano già teletrasportarsi da
  una città a qualsiasi distretto navale (niente più ammiragli bloccati in un lago di una casella).

### Correzioni dell'interfaccia
- I pulsanti di attacco (città, accampamento, attacco nucleare da un silo) vengono disegnati sopra le
  bandiere delle unità, dove si possono cliccare.
- L'attacco a distanza da una città non selezionata richiede un clic, non due.
- In modalità mira per l'attacco a distanza di una città o di un distretto, cliccare sulla bandiera
  di un'unità nemica la attacca.
- La visuale non si centra più al clic su un pulsante di attacco (si può riattivare).
- Il contatore dei turni di produzione ora include i bonus di governo: fascismo, autocrazia e
  repubblica mercantile.
- Una meraviglia tolta dalla coda di produzione dopo due scambi viene rimessa.
- La produzione in corso si aggiorna correttamente quando si passa a un'altra città.
- Le rese dell'impero nella barra superiore si aggiornano dopo un'azione di un governatore o un grande
  personaggio.
- *(sperimentale)* Le scorciatoie da tastiera non smettono più di funzionare quando una schermata di
  diplomazia chiude il menu Esc, o dopo un doppio popup del Congresso.
- Esc ora chiude correttamente la schermata degli inviati: gli inviati in sospeso non bloccano più la
  fine del turno.
- *(sperimentale)* La lente del colono viene ridisegnata dopo la fondazione di una città, e le caselle
  troppo vicine a una città appaiono come vietate.
- Invio segue il tuo tasto per «Azione successiva»: puoi quindi toglierlo da lì per assegnarlo a
  un'altra scorciatoia. Maiusc+Invio forza sempre la fine del turno.
- L'area di movimento viene mostrata per un'unità su un distretto difendibile nemico.
- *(sperimentale)* I segnalini della mappa non vengono più ricostruiti quando nulla è cambiato: meno
  rallentamenti in multigiocatore.

### Mod dell'interfaccia integrate
Ognuna si può disattivare nella scheda BetterGameExperience. Se hai anche la mod del Workshop, la versione
integrata in BGE la sostituisce.
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
    aggiornati una sola volta. Il turno misurato è stato più rapido di 2,4 s (−82 % sulla cascata):
    molto meno lag quando le città-stato cambiano schieramento in una guerra.
  - Due cache accelerano il turno dell'IA: miglioramenti costruibili e rese delle caselle.
  - I risultati sono identici a quelli del gioco base: nessun rischio con chi non ha la mod.
- L'animazione degli attacchi nucleari è 20 volte più rapida con il combattimento rapido: circa 0,6 s
  invece di 11 s. Solo visualizzazione.
- Il limitatore di fotogrammi offre 45, 75, 90 e 100 FPS.

### Limiti noti
- La mod è fatta per la versione ufficiale attuale del gioco. Dopo un aggiornamento del gioco, le
  funzioni interessate si disattivano da sole finché BetterGameExperience non viene aggiornato.
- Un giocatore che entra a partita in corso senza la mod, mentre le correzioni **[tutti i
  giocatori]** sono attive, si desincronizza. L'host viene avvisato.
- Le correzioni segnate *(sperimentale)* non sono ancora state provate in una partita reale, o il
  loro effetto non è ancora stato misurato del tutto.
- Le traduzioni diverse da inglese e francese non sono ancora state riviste da madrelingua. Le
  correzioni sono benvenute.
