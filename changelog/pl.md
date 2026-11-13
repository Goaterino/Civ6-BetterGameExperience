# Lista zmian

[English](../CHANGELOG.md) · inne języki: zobacz [listę zmian po angielsku](../CHANGELOG.md).

## Następna wersja

### Poprawki gry wieloosobowej [wszyscy gracze]
- **Obserwatorzy (Better Spectator Mod)**: przebudowa obserwatorów z 0.9.0 zostaje wycofana, bo
  rozgrywki z obserwatorem były mniej stabilne. BetterGameExperience nie zmienia już poziomu
  cywilizacji obserwatorów ani nie zastępuje skryptów Better Spectator Mod, a ustawienie « Better
  Spectator Mod dla obserwatorów » znika. Obserwatorzy znów liczą się jako główne cywilizacje dla
  skryptów, zwycięstw i wyniku, jak przy samym Better Spectator Mod. Turystyka i głosowania Kongresu
  Światowego nadal ich pomijają.
- **Zachowane dla obserwatorów**: zmiana obserwowanego gracza nadal nie powoduje desynchronizacji, a
  tura obserwatora nadal kończy się sama po pauzie MPH. Obie poprawki dotyczą teraz oryginalnych
  skryptów Better Spectator Mod.

## BetterGameExperience 0.9.1

Zgodna z 0.9.0 w grze wieloosobowej: gracze z 0.9.0 i 0.9.1 grają razem, ze wszystkimi poprawkami.

### Poprawki interfejsu
- **Ekran raportów**: karty Plony i Miasta znów działają. Obliczanie efektu kart doktryn, ładowane
  na tym samym ekranie, zastępowało część jego danych i je psuło. Karty Doktryny i Miasta-państwa
  nie zawodzą już na wierszu z punktami wybitnych postaci, a sortowanie kolumny karty Miasta nie
  zawodzi już, gdy karta Plony nie została wcześniej otwarta.
- **Tańsze dzielnice**: pokazywana zniżka to teraz ta, którą stosuje gra. Gra aktualizuje koszt
  dzielnicy tylko w określonych momentach (nowa technologia lub idea, nowe miasto, dzielnica tego
  samego typu postawiona lub ukończona), więc lista mogła pokazywać zniżkę, którą gra dawała dopiero
  kilka tur później.

## BetterGameExperience 0.9.0

Zawiera wszystko z wersji przedpremierowej 0.8.1. W grze wieloosobowej wszyscy ludzcy gracze muszą
mieć 0.9.0, aby działały poprawki **[wszyscy gracze]**: poczekalnia, w której są 0.8.x i 0.9.0, je
wyłącza.

### Poprawki gry wieloosobowej [wszyscy gracze]
- **Obserwatorzy (Better Spectator Mod)**: nie liczą się już jako główne cywilizacje i znikają z
  list graczy w skryptach, zwycięstwach i wyniku. BetterGameExperience zawiera do tego własne wersje
  skryptów Better Spectator Mod. Wkład: banipouz.
- **Desynchronizacja przez obserwatora**: zmiana gracza, którego ogląda obserwator, nie
  desynchronizuje już gry (Better Spectator Mod zapisywał ją w danych gry, tylko na komputerze
  obserwatora).
- **Obserwatorzy na wstędze przywódców**: gracze nie widzą już obserwatorów na wstędze w prawym
  górnym rogu, gdzie pojawiali się jak SI, z portretem « ? », nazwą i plonami.
- **Tura obserwatora po pauzie**: po pauzie MPH tura obserwatora znów kończy się sama, zamiast
  czekać, aż zakończy ją ręcznie.

### Tryb wieloosobowy
- **Uszkodzone wiadomości sieciowe**: uszkodzona wiadomość, widziana przy rozłączeniu gracza, mogła
  zamrozić grę, aż po kilku minutach zabrakło pamięci. Niemożliwe wartości takiej wiadomości są
  teraz odrzucane i gra toczy się dalej; wiersz na czacie mówi o tym wszystkim. Uszkodzona
  konfiguracja gry lub gracza nie jest już stosowana, zamiast kasować konfigurację. Chroni graczy,
  którzy mają BetterGameExperience. Wkład: yodha.
- **No First/Last moves z obserwatorem jako hostem**: tura zaczyna się teraz, gdy tylko gracze są
  gotowi, zamiast trzymać ich do limitu bezpieczeństwa.

### Nowości
- **Postęp badań i ustroju**: w panelu w lewym górnym rogu, obok nazwy bieżącej technologii i idei,
  **x/y** pokazuje już włożoną naukę (kulturę) i całkowity koszt, a **(+z)** nadwyżkę nauki
  (kultury), którą gra zachowuje z technologii (idei) ukończonej ponad jej koszt. Można to wyłączyć
  w ustawieniach.
- **Disable Advanced UI indicators (BGE)**: nowa opcja gry tuż pod No First/Last moves, domyślnie
  wyłączona. Gdy host ją zaznaczy, żaden gracz nie widzi postępu badań i ustroju (**x/y (+z)**) ani
  produkcji zachowanej przez miasta, niezależnie od własnych ustawień. Jest odczytywana w każdej
  grze: w grze bez tej opcji wskaźniki wracają, bez ponownego uruchamiania Civ VI.
- **Anulowanie produkcji z banera miasta**: prawy klik na ikonę produkcji w banerze jednego z twoich
  miast usuwa bieżący element, tak jak w panelu produkcji.
- **Tańsze dzielnice**: na liście produkcji dzielnica o obniżonym koszcie (masz mniej dzielnic tego
  typu niż twoja średnia) pokazuje zniżkę, na przykład **-40%**. Odświeża się od razu po
  umieszczeniu dzielnicy, bo nowa dzielnica może zakończyć zniżkę. Dzielnice, które nie liczą się do
  średniej, jak wietnamski Thành, też są oznaczone.
- **Zachowana produkcja**: baner miasta, panel miasta i panel produkcji pokazują produkcję, którą
  miasto trzyma w zapasie (nadwyżka z ukończonego elementu albo tura bez produkcji; gra zachowuje
  najwyżej jedną turę) i która dochodzi do jego produkcji w następnej turze. W banerze miasta bez
  produkcji pojawia się jako **+N**.
- **BGE Check**: w poczekalni gry wieloosobowej host ma przycisk **BGE CHECK**, obok MOD CHECK z
  MPH, który pyta każdego gracza o jego wersję BetterGameExperience i wypisuje wynik na czacie: ta
  sama wersja, inna wersja, zainstalowany, ale wyłączony, albo brak odpowiedzi.
- **Wiadomość o dołączeniu**: po otwarciu poczekalni z włączonym BetterGameExperience czat to
  ogłasza, z dokładną wersją.

### Poprawki interfejsu
- **Panel miasta po usunięciu produkcji**: panel w prawym dolnym rogu odświeża się po usunięciu
  jedynego elementu kolejki, zamiast nadal go pokazywać.
- **Prawy klik na bieżącą produkcję**: usuwa ją teraz także w karcie Produkcja, nie tylko przy
  otwartej kolejce.
- **Poczekalnia MPH**: poza draftem lista graczy przesuwa się w górę, w puste miejsce, które MPH
  rezerwuje na draft, a samotna « . » nad nią we wczytanej grze znika.
- **Ekran dyplomacji**: po spotkaniu gracza, którego drużyny brakowało na liście drużyn gry, ikony
  przywódców zawodziły co turę, a otwarcie ekranu dyplomacji mogło na dobre ukryć cały interfejs.
  Ikony znów działają, a jeśli ekran dyplomacji się nie otworzy, interfejs wraca.
- **Imhotep**: jego efekt pokazuje teraz wartość dla tempa gry (87 albo 175 dla cudu starożytnego
  lub klasycznego, przy tempie Online), na ekranie wielkich ludzi i w Cywilopedii, zamiast zawsze
  175.
- **Pauza MPH**: menu MULTIPLAYER HELPER i głosowanie nad remapem pojawiają się teraz nad ekranem
  pauzy MPH, z klikalnymi przyciskami.
- **Górny pasek**: średnie udogodnienia na miasto są zaokrąglane do najbliższej liczby całkowitej, a
  nie w dół.
- **Zgłoś błąd (BGE)**: zapis gry jest teraz dołączany do zgłoszenia; wysyłane były tylko autozapisy.

### Ustawienia
- Nowe funkcje interfejsu (anulowanie z banera miasta, tańsze dzielnice, zachowana produkcja, postęp
  badań i ustroju) oraz skrypty Better Spectator Mod mają każde ustawienie w zakładce
  BetterGameExperience, domyślnie włączone.

## BetterGameExperience 0.8.0

W grze wieloosobowej wszyscy ludzcy gracze muszą mieć 0.8.0. Poprawka umów na koniec tury chroni
tylko tych, którzy ją mają: jeden gracz, który został przy 0.7.x, może sprowadzić zablokowaną turę na
wszystkich. W poczekalni gospodarz widzi na czerwono, kto ma inną wersję, a poczekalnia, w której są
0.7.x i 0.8.0, wyłącza poprawki **[wszyscy gracze]**.

### Poprawki gry wieloosobowej [wszyscy gracze]
- **Tura w pętli po umowie między ludźmi**: umowa zaproponowana lub przyjęta tuż pod koniec twojej
  tury mogła dotrzeć do gospodarza po końcu tury. Następna tura zaczynała się wtedy w kółko u
  niektórych graczy, trzeba było wracać do poczekalni, a gra często się desynchronizowała. Taka umowa
  nie jest już wysyłana, gdy twój zegar się skończył ani gdy żaden człowiek nie ma już aktywnej tury:
  mówi ci o tym wiersz na czacie. Umowy z SI się nie zmieniają.

### Nowości
- Przycisk **Zgłoś błąd (BGE)** w menu Esc, pod Opcjami. Opisz problem: BGE wysyła zapis gry,
  ostatnie autozapisy i dzienniki gry (bez twojej nazwy użytkownika Windows), a wiersz na twoim czacie
  podaje numer zgłoszenia. Pozostali gracze widzą na czacie, że zgłoszenie jest wysyłane. Jeśli
  wysyłanie się nie uda, zip trafia na twój Pulpit.

### Poprawki interfejsu
- **Efekt kart ustrojowych**: Racjonalizm, Wielka opera, Simultaneum i Wolny rynek liczą też stałe
  plony dodane do budynków, które wzmacniają (gubernatorzy…), a nie tylko ich plon podstawowy.
  Premie od miast-państw dla tych budynków są pomijane, jak w grze, która ich nie mnoży.
- **Wybór szlaków handlowych**: dla szlaku wewnętrznego widać teraz złoto, które zdolność Poundmakera
  daje miastu docelowemu (+1 za każdy obóz i każde pastwisko), z wierszem podpowiedzi, który nazywa
  to miasto.
- **Górny pasek**: przycisk populacji pokazuje też średnie udogodnienia netto na miasto (na zielono
  od +3, na czerwono poniżej 0).
- **Menu główne**: wersja BetterGameExperience jest widoczna w lewym dolnym rogu, po wersjach gry
  i MPH (`… - MPH / … - BGE / 0.8.0`).

### Ustawienia
- Opcja gry **„No First/Last moves (BGE)”**: teraz domyślnie zaznaczona. Gospodarz wciąż może ją
  odznaczyć.

## BetterGameExperience 0.7.0

W grze wieloosobowej poprawki **[wszyscy gracze]** wymagają, by wszyscy ludzcy gracze mieli 0.7.0:
poczekalnia, w której są 0.6.x i 0.7.0, je wyłącza.

### Poprawki rozgrywki [wszyscy gracze]
- **Jednostki w stosie**: dwie jednostki wojskowe tej samej klasy mogą znaleźć się na tym samym polu
  (jednostka kupiona w zajętym mieście, przerwany ruch, teleportacja…). Nadmiarowa jednostka nie
  liczy się już do premii za oskrzydlenie i wsparcie: dwie jednostki w stosie obok celu dają +2, a
  nie +4. Obrona miast się nie zmienia.

### Kongres Światowy
- **Usunięte**: poprawki z 0.6.x, które nie pozwalały zakończyć tury w trakcie głosowania (brak
  automatycznego końca tury, brak wymuszonego końca tury przy pustej liście rezolucji, 240 s na
  zegarze MPH). Zegar i koniec tury gry oraz MPH znów działają w trakcie głosowania. Przycisk
  **Wróć do głosowania**, ekran powitalny raz na sesję i ponowne przeliczanie dóbr luksusowych
  zostają.

### Poprawki interfejsu
- **Ekran miast-państw**: wysłannicy rozmieszczeni, ale jeszcze niepotwierdzeni, nie przepadają już przy
  odświeżeniu ekranu (wysłannik innego gracza, misja, wojna lub pokój). Są usuwani tylko wtedy, gdy
  miasto-państwo zostanie zniszczone albo twoja drużyna wypowie mu wojnę.
- **Ekran umowy**: przycisk dostępnego złota znów działa. Po wcześniejszej umowie mógł pozostać
  nieklikalny (błąd gry).
- **Cała mapa od początku** (domyślnie włączone): minimapa i kamera obejmują całą mapę od pierwszej
  tury, nie trzeba już znaczników na krawędziach mapy. Niezbadane obszary pozostają ukryte. Działa
  tylko lokalnie.
- **Zasięg ruchu jednostki na wrogiej dzielnicy**: koniec z przycięciami przy jej zaznaczaniu
  (nieukończony wrogi obóz), strefy kontroli (czerwone pola) są widoczne, a pola wody,
  na które jednostka nie może jeszcze wejść (wybrzeże przed zaokrętowaniem, ocean przed
  Kartografią), nie są już proponowane.

### Ustawienia
- Przycisk **Zresetuj BGE** w zakładce BetterGameExperience: przywraca wszystkie ustawienia do
  wartości domyślnych.
- Nowe zaawansowane ustawienie diagnostyczne, domyślnie wyłączone: **Pomiar kosztu zasięgu ruchu (w czasie)**.

## BetterGameExperience 0.6.1

Zawiera wszystko z wersji przedpremierowej 0.6.0. W grze wieloosobowej poprawki **[wszyscy gracze]**
wymagają, by wszyscy ludzcy gracze mieli 0.6.0 lub 0.6.1: poczekalnia, w której są 0.5.0 i 0.6.x,
je wyłącza.

### Instalacja i aktualizacje
- Po instalacji instalator nie pokazuje już opcji uruchamiania Steam: informuje, że wszystko jest
  gotowe, i można go zamknąć. Powiadomienie o aktualizacji w grze wystarcza, by być na bieżąco. Już
  dodana opcja uruchamiania nadal działa.
- Instalator nazywa się teraz `BetterGameExperience_V<version>.exe`. Po uruchomieniu usuwa starsze
  instalatory z tego samego folderu (zwykle Pobrane).

### Tryb wieloosobowy
- **„No First/Last moves (BGE)”** **[wszyscy gracze]**: zostajesz na ekranie zmiany tury, razem z
  przyciskiem końca tury, dopóki gospodarz nie rozpocznie tury, zamiast widzieć otwartą turę z
  odrzucanymi rozkazami.

### Kongres Światowy
- *(eksperymentalne)* **[wszyscy gracze]** Gdy czekasz na innych graczy, przycisk **Wróć do
  głosowania** ponownie otwiera okno głosowania z twoimi głosami i właściwą ilością przychylności
  dyplomatycznej.
- *(eksperymentalne)* Tura nie kończy się już wbrew twojej woli podczas głosowania: bez
  automatycznego końca tury, bez wymuszonego końca tury przy pustej liście rezolucji, a zegar MPH daje
  przewidziane 240 s na głosowanie.
- *(eksperymentalne)* **[wszyscy gracze]** Dobra luksusowe objęte zakazem są przeliczane na każdej
  sesji.
- Ekran powitalny Kongresu Światowego i jego dźwięk pojawiają się tylko raz na sesję, zamiast wracać
  za każdym razem, gdy inny gracz wraca do głosowania.

### Poprawki interfejsu
- Z Better Spectator Mod górny pasek znów pokazuje zasoby strategiczne twoich współgraczy oraz
  zasoby, które odblokowuje współgracz.

## BetterGameExperience 0.5.0: pierwsza wersja publiczna

BetterGameExperience (BGE) poprawia i przyspiesza Civilization VI: Gathering Storm za pomocą dwóch bibliotek DLL
umieszczonych obok plików gry. **Żaden plik gry nie jest zastępowany**, więc ani weryfikacja plików
w Steam, ani aktualizacje gry go nie usuwają. Działa z DirectX 11 i DirectX 12, z BBG, Better
Spectator Mod i MPH.

Wszystkie zmiany są wymienione poniżej, według kategorii. Zmiany oznaczone **[wszyscy gracze]**
zmieniają zasady gry: włączają się tylko wtedy, gdy wszyscy ludzcy gracze w poczekalni mają tę samą
wersję BetterGameExperience. Cała reszta dotyczy tylko twojego komputera. Można bezpiecznie grać z osobami,
które nie mają moda.

### Instalacja i aktualizacje
- Instalator `BetterGameExperience.exe`: instalacja, aktualizacja, naprawa i odinstalowanie. Znajduje grę
  w bibliotekach Steam i przed skopiowaniem czegokolwiek sprawdza podpisany manifest oraz SHA-256
  każdego pliku.
- Obok plików gry dodawane są dwa pliki: `winmm.dll` i `BGD_GameCore_XP2.dll`. Starsze instalacje
  (0.3.0 i wcześniejsze) są automatycznie sprzątane. Plik `winmm.dll` innego moda nigdy nie jest
  nadpisywany.
- Powiadomienie o aktualizacji w menu głównym, także w menu MPH. **Aktualizuj** zamyka grę, instaluje
  nową wersję i proponuje ponowne uruchomienie gry.
- Pole „Wersja testowa” w instalatorze do testowania nadchodzących wersji, z powrotem do wersji
  stabilnej jednym kliknięciem.
- Opcjonalna opcja uruchamiania Steam, która sprawdza aktualizacje przy każdym uruchomieniu gry.
- Instalator w 11 językach, tych samych co Better Balanced Game. Używa języka Windows, a inny można
  wybrać na dole okna.

### Ustawienia
- Nowa karta **BetterGameExperience** w opcjach gry: główny przełącznik, który wyłącza wszystko bez
  odinstalowywania, ustawienie dla każdej funkcji i opcje zaawansowane domyślnie ukryte. Informuje,
  gdy zmiana wymaga ponownego uruchomienia.
- Podziękowania dla autorów wbudowanych modów interfejsu.
- Wszystkie teksty w grze są dostępne w 11 językach Better Balanced Game: angielskim, francuskim,
  niemieckim, hiszpańskim, włoskim, japońskim, koreańskim, polskim, portugalskim (Brazylia), rosyjskim
  i chińskim uproszczonym.

### Tryb wieloosobowy
- **Sprawdzanie w poczekalni**: każdy komputer zgłasza swoją wersję BetterGameExperience, a host decyduje za
  wszystkich. Wiadomości czatu z przedrostkiem `BGE -` mówią, komu brakuje moda lub kto ma inną
  wersję. Ponowne dołączenia są obsługiwane. Jeśli ktoś dołączy bez moda, gdy poprawki **[wszyscy
  gracze]** są włączone, host dostaje ostrzeżenie.
- Wiadomości czatu `BGE -` są wysyłane po angielsku, aby gracze bez moda mogli je przeczytać. Gracze
  z modem widzą je w języku swojej gry.
- **Opcja gry „No First/Last moves (BGE)”** **[wszyscy gracze]**:
  - Wszyscy gracze zaczynają turę razem. Rozkazy są wstrzymane, a zegar stoi, dopóki wszyscy nie
    skończą wczytywania (najwyżej 60 s).
  - Tura kończy się dla wszystkich, gdy upłynie czas na zegarze hosta.
  - Stara synchronizacja tury z MPH zostaje wyłączona.
- Wczytane gry w poczekalni MPH: miasta-państwa nie powodują już błędu „Nieprawidłowe opcje gracza”
  ani wyszarzonego przycisku Gotowy.

### Poprawki rozgrywki [wszyscy gracze]
- Pola uprawiane: miasto nie uprawia już pola 2/1, gdy wolne jest pole 2/2 (priorytet żywności
  i produkcji).
- Obserwatorzy z Better Spectator Mod nie są już liczeni w turystyce. Nie mogą głosować w Kongresie
  Światowym ani być celem rezolucji. Nadal mogą oglądać ekran Kongresu.
- Jednostki morskie mogą teleportować się do miasta z dowolnej dzielnicy morskiej (port, park wodny,
  kanał…), a nie tylko z centrum miasta, tak jak już mogły teleportować się z miasta do dowolnej
  dzielnicy morskiej (koniec z admirałem uwięzionym na jeziorze z jednego pola).

### Poprawki interfejsu
- Przyciski ataku (miasto, dzielnica wojskowa, atak jądrowy z silosu) są rysowane nad flagami
  jednostek, gdzie można je kliknąć.
- Atak dystansowy z niewybranego miasta wymaga jednego kliknięcia zamiast dwóch.
- W trybie celowania ataku dystansowego miasta lub dzielnicy kliknięcie flagi wrogiej jednostki
  atakuje ją.
- Kamera nie centruje się już po kliknięciu przycisku ataku (można to ponownie włączyć).
- Licznik tur produkcji uwzględnia teraz premie ustroju: faszyzm, autokrację i republikę kupiecką.
- Cud, który wypadł z kolejki produkcji po dwóch zamianach, wraca do niej.
- Bieżąca produkcja poprawnie się odświeża po przejściu do innego miasta.
- Plony imperium na górnym pasku odświeżają się po działaniu gubernatora lub wielkim człowieku.
- *(eksperymentalne)* Skróty klawiszowe nie przestają działać, gdy ekran dyplomacji zamknie menu Esc,
  ani po podwójnym okienku Kongresu.
- Esc poprawnie zamyka teraz ekran wysłanników: oczekujący wysłannicy nie blokują już końca tury.
- *(eksperymentalne)* Soczewka osadnika jest odrysowywana po założeniu miasta, a pola zbyt bliskie
  miasta są pokazywane jako zabronione.
- Enter działa zgodnie z twoim klawiszem „Następna akcja”: można go więc stamtąd usunąć i przypisać
  do innego skrótu. Shift+Enter nadal wymusza koniec tury.
- Zasięg ruchu jest widoczny dla jednostki stojącej na wrogiej dzielnicy obronnej.
- *(eksperymentalne)* Znaczniki mapy nie są już przebudowywane, gdy nic się nie zmieniło: mniej
  spowolnień w trybie wieloosobowym.

### Wbudowane mody interfejsu
Każdy można wyłączyć na karcie BetterGameExperience. Jeśli masz też moda z Warsztatu, wersja wbudowana w
BGE go zastępuje.
- **Ładunki budowniczych** na flagach jednostek (na podstawie Better Builder Charges Tracking, autor:
  wltk).
- **Pozostałe tury szlaków handlowych** na flagach twoich kupców (nowość).
- Przycisk **Wyczyść powiadomienia** (na podstawie NotificationClear, autor: 336tarot).
- **Rozszerzony górny pasek**: żywność, produkcja, populacja i dobra luksusowe, ze szczegółami dla
  każdego miasta (na podstawie TopPanel Extension Pro, autor: 号码菌 / Azukii).
- **Efekt kart doktryn**: każda karta pokazuje swój rzeczywisty zysk dla twojego imperium (na
  podstawie Extended Policy Cards, autor: Aristos). Better Report Screen nie jest już potrzebny.
  Poprawione wartości dla:
  - Zwierzchnictwa, Medyny, Monastycyzmu i Prestiżu obywatelskiego;
  - Zaciągu i Levée en masse;
  - kart Pretorium i Podatki kolonialne;
  - punktów wielkich ludzi, teraz pokazywanych jako [ikona]+x.
- **Ulepszony ekran raportów** z kartami Umowy, Jednostki, Doktryny, Miasta-państwa i Miasta (na
  podstawie Better Report Screen, autor: Infixo), z tymi samymi poprawkami.

### Wydajność i wygoda
- Szybsze przetwarzanie tury:
  - Podczas kaskady wypowiedzeń wojny dane miast i dyplomacji są odświeżane tylko raz. Zmierzona
    tura była szybsza o 2,4 s (−82 % na kaskadzie): znacznie mniej lagów, gdy miasta-państwa
    zmieniają strony podczas wojny.
  - Dwie pamięci podręczne przyspieszają turę SI: możliwe ulepszenia i plony pól.
  - Wyniki są identyczne jak w podstawowej grze: bezpieczne z graczami bez moda.
- Animacja ataków jądrowych jest 20 razy szybsza przy włączonej szybkiej walce: około 0,6 s zamiast
  11 s. Tylko wyświetlanie.
- Ogranicznik klatek oferuje 45, 75, 90 i 100 FPS.

### Znane ograniczenia
- Mod jest przeznaczony dla obecnej oficjalnej wersji gry. Po aktualizacji gry odpowiednie funkcje
  same się wyłączają, dopóki BetterGameExperience nie zostanie zaktualizowane.
- Gracz, który dołączy w trakcie rozgrywki bez moda, gdy poprawki **[wszyscy gracze]** są włączone,
  ulegnie desynchronizacji. Host dostaje ostrzeżenie.
- Poprawki oznaczone *(eksperymentalne)* nie zostały jeszcze przetestowane w prawdziwej rozgrywce
  albo ich efekt nie został jeszcze w pełni zmierzony.
- Tłumaczenia inne niż angielskie i francuskie nie zostały jeszcze sprawdzone przez rodzimych
  użytkowników języka. Poprawki są mile widziane.
