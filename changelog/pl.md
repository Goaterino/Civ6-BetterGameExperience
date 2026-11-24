# Lista zmian

[English](../CHANGELOG.md) · inne języki: zobacz [listę zmian po angielsku](../CHANGELOG.md).

## BetterGameExperience 0.15.0

W grze wieloosobowej wszyscy ludzcy gracze muszą mieć 0.15.0, aby działały poprawki
**[wszyscy gracze]**: poczekalnia, w której są 0.14.0 i 0.15.0, je wyłącza.

### Wersje gry
- **Civilization VI 1.0.12.72 nie jest już obsługiwana**: jej gracze przeszli na 1.0.12.74. W
  1.0.12.72 BetterGameExperience nie instaluje żadnej poprawki i informuje o tym w menu głównym, w
  poczekalni i na przycisku zgłoszenia błędu.
- **1.0.12.58 z plikami gry z czerwca 2024**: od 0.14.0 to połączenie nie dostawało żadnej poprawki.
  Znów jest rozpoznawane.

### Poprawki gry wieloosobowej [wszyscy gracze]
- **Tura nieobecnego gracza**: gdy inny gracz kończył turę nieobecnego, gra mogła sama ją ponownie
  otworzyć (jednostka z pozostałym ruchem), a nikt już jej nie kończył: tura stała w miejscu. Tura
  zakończona przez innego gracza pozostaje teraz zakończona.
- **Widoczność miasta-państwa po wojnie**: gracz, który znów zostawał zwierzchnikiem miasta-państwa
  po wojnie z nim (częste w grach Teamer), nigdy nie odzyskiwał jego widoczności. Teraz ona wraca, a
  rozgrywki już dotknięte są naprawiane w następnej turze.
- **Niewidoczne wioski plemienne po wczytaniu**: od 0.14.0 wioski plemienne we mgle znikały z mapy
  po ponownym wczytaniu lub resynchronizacji. Znów są widoczne.
- **Większe fragmenty sieciowe**: duże wiadomości sieciowe (zapisy gry, kontrole synchronizacji) są
  teraz wysyłane we fragmentach po 16 KB zamiast 1,2 KB, tylko w rozgrywce, w której każdy
  połączony gracz ma 0.15.0. Gracz bez BetterGameExperience lub z inną wersją od razu przywraca
  rozmiar gry.

### Tryb wieloosobowy
- **Szybsze kontrole synchronizacji u hosta**: na koniec każdej tury host nie przelicza już tych
  samych danych synchronizacji dla każdego gracza. Do tego tylko host potrzebuje 0.15.0.
- **Długie zmiany tury po powrocie do gry**: dwie powyższe zmiany dotyczą długich zmian tury po
  powrocie gracza do gry (do 45 s na turę). Zysk trzeba jeszcze potwierdzić w prawdziwych
  rozgrywkach: dajcie znać, co widzicie.
- **Zamiana miejsc z graczem bez BetterGameExperience**: zamiana miejsc w poczekalni mogła sprawić,
  że host liczył gracza bez BetterGameExperience jako wyposażonego i włączał mu poprawki. Każde
  ogłoszenie jest teraz powiązane z kontem Steam gracza.
- **Gotowość zachowana w poczekalni**: zmiana hosta, która nie zmienia ustawień rozgrywki, nie
  odbiera już graczom gotowości. Gdy ustawienia gracza wciąż różnią się od ustawień hosta, kiedy host
  jest gotowy, czat o tym informuje i prosi tego gracza o ponowne dołączenie.
- **Poczekalnia po anulowanym restarcie (MPH)**: po anulowaniu restartu uruchomionego z poczekalni
  następna rozgrywka utworzona lub wczytana z poczekalni była wczytywana dwa razy przez każdego
  gracza. Teraz jest wczytywana raz.
- **Poczekalnia zapisanej gry**: dołączenie do niej mogło pokazywać „nieprawidłowe opcje gracza" dla
  przywódców z DLC lub modów, dopóki nie otwarto ponownie konfiguracji gry. Lista graczy jest teraz
  odbudowywana.

### Kongres Światowy
- **Drugi powrót do głosowania**: gracz, który drugi raz wrócił do głosowania, nie mógł już oddać
  wszystkich głosów. Głosy już wysłane są teraz zachowywane.

### Szybsze wczytywanie
- **Użycie procesora na ekranie wczytywania**: od 0.14.0 ekran wczytywania mógł działać dalej bez
  przerwy między klatkami i bez VSync po przygotowaniu mapy, przy pełnym obciążeniu procesora. Ta
  szybka faza kończy się teraz, gdy tylko teren jest gotowy (najwyżej 3 s).

### Interfejs
- **Szansa powodzenia szpiegów**: szansa pokazywana przy wyborze misji szpiega uwzględnia teraz
  ucieczkę: sukces wymagający ucieczki opłaca się tylko, jeśli szpieg ucieknie. Zakłada najlepszą
  drogę ucieczki i brak kontrwywiadu, jak gra. Domyślnie włączone (ustawienia interfejsu).
- **Sygnały drużyny**: głośność i skróty sygnałów są zachowywane między wczytaniami i ponownymi
  uruchomieniami gry.
- **Karta ustawień**: ponowne otwarcie karty BetterGameExperience mogło pokazywać stare wartości.

## BetterGameExperience 0.14.0

W grze wieloosobowej wszyscy ludzcy gracze muszą mieć 0.14.0, aby działały poprawki
**[wszyscy gracze]**: poczekalnia, w której są 0.13.0 i 0.14.0, je wyłącza.

### Instalacja i aktualizacje
- **Podpisane pliki**: instalator i obie biblioteki DLL BetterGameExperience mają teraz certyfikat
  podpisywania kodu (Certum, programista open source). Windows pokazuje wydawcę we właściwościach
  pliku i w oknie instalatora, a ostrzeżenia SmartScreen i programów antywirusowych powinny być
  rzadsze.

### Wersje gry
- **Civilization VI 1.0.12.58**: BetterGameExperience rozpoznaje teraz tę starszą wersję gry, w DX12 i
  DX11. Każda wersja gry ma własne adresy, a gra, której pliki nie pasują do jej wersji, nie
  instaluje żadnej poprawki.
- **Nieobsługiwana wersja gry**: gracz, którego wersja gry nie jest obsługiwana przez
  BetterGameExperience, jest ogłaszany na czacie po dołączeniu (komunikat mówił wcześniej, że jego
  kopia nie jest aktualną wersją ze Steam).

### Poprawki gry wieloosobowej [wszyscy gracze]
- **Mniej desynchronizacji po wczytaniu**: gra wczytana na jednym komputerze liczyła nieco inaczej niż
  gra toczona na pozostałych. Zapamiętane ulepszenia graczy były przepisywane przy wczytaniu,
  odbiorniki zdarzeń działały w innej kolejności, SI na stałe zachowywała preferencje swojego
  przywódcy (Sejong: -75% punktów Wielkiego Proroka), a jednostki dawane przy tworzeniu dzielnicy
  (bonusy poziomu trudności SI, start w późniejszej erze) powstawały w innej kolejności. Wczytana gra
  liczy teraz tak samo jak toczona. Wkład OzrielTheLost.
- **Te same obliczenia na każdym procesorze**: niektóre obliczenia (funkcje wykładnicze, potęgi) mogły
  zaokrąglać się inaczej na procesorach z instrukcjami FMA3 i bez nich. Wszystkie komputery liczą je
  teraz tak samo. Wkład OzrielTheLost.
- **Zamiana jednostek**: zamiana dwóch jednostek mogła zostawić jedną w połowie drogi, na jednym polu
  z inną jednostką (szermierz na przyczajonym tygrysie). Obie jednostki kończą teraz swój ruch.

### Gra wieloosobowa
- **Desynchronizacje naprawiane bez wczytywania**: gdy obiekt pozostaje rozsynchronizowany, host
  wysyła teraz wszystkie jego dane, a gra gracza naprawia go na miejscu, zamiast odpływać aż do
  pełnego wczytania. Do tego wersji 0.14.0 potrzebuje tylko host. Wkład OzrielTheLost.

### Kongres Światowy
- **Tura kongresu po wymuszonym końcu tury**: gdy jednoczesny koniec tury wymuszał zakończenie tury
  gracza, tura kongresu mogła skończyć się dwie sekundy po otwarciu, bez głosowania. Tura kongresu
  zaczyna się teraz z pełnym zegarem, okno głosowania otwiera się ponownie, jeśli tura skończyła się
  bez głosu, a przycisk kongresu pokazuje bieżącą sesję.

### Szybsze wczytywanie
- **Szybsze ponowne wczytywanie**: przy tych samych modach ponowne wczytanie zachowuje etap
  konfiguracji i zostawia teksty modów bez zmian, ostatnia faza wczytywania nie czeka już między
  krokami, a czekanie na VSync jest wyłączone także w DX11. Zmierzone na małej mapie: z 13 s do 9 s.
  Ustawienia w opcjach zaawansowanych, domyślnie włączone. Wkład OzrielTheLost.

### Interfejs
- **Skróty na ekranie umowy**: w kolumnie „dostępne” Shift+klik dodaje tyle, ile umowa przyjmie, a
  Ctrl+klik dodaje dziesięć razy tyle co klik, dla surowców, złota, złota na turę i przychylności.
  Domyślnie włączone. Wkład huchenlei.
- **Odznaka zniżki dzielnic**: po postawieniu dzielnicy ze zniżką w jednym mieście inne, już oglądane
  miasto nadal pokazywało zniżkę, choć dzielnica kosztowała pełną cenę.
- **Nadwyżka produkcji**: „+N” na banerze miasta zostawało po tym, jak tura je wydała.

### Ustawienia
- **Nowości interfejsu z aktualizacji**: nowe ustawienie na początku sekcji Interfejs. Na
  „Wyłączone” nowości interfejsu z kolejnych aktualizacji przychodzą wyłączone, a te, które masz,
  zachowują obecny stan.

## BetterGameExperience 0.13.0

W grze wieloosobowej wszyscy ludzcy gracze muszą mieć 0.13.0, aby działały poprawki
**[wszyscy gracze]**: poczekalnia, w której są 0.12.0 i 0.13.0, je wyłącza.

### Aktualizacja gry 1.0.12.74
- **Aktualizacja Civilization VI z 19 listopada (1.0.12.74)**: BetterGameExperience rozpoznaje teraz
  zaktualizowaną grę, w DX12 i DX11. Wcześniejsze wersje nie instalują w niej żadnej ze swoich
  poprawek. Wkład yodha.

### Poprawki gry wieloosobowej [wszyscy gracze]
- **Gracz, który się rozłącza**: gra oddawała rozłączonego gracza SI na każdym komputerze osobno, bez
  żadnej wiadomości między nimi, więc komputery mogły rozegrać tę turę inaczej i się rozsynchronizować.
  Miejsce pozostaje teraz ludzkie, a host kończy turę nieobecnego gracza, gdy wszyscy połączeni
  ludzie zakończą swoją. Wyrzucony gracz nadal przechodzi pod SI. Wkład yodha.
- **Ponowne wczytanie u wszystkich**: gdy gra jednego gracza przestaje się zgadzać, host sam
  rozpoczyna ponowne wczytanie i wszyscy gracze wczytują tę samą kopię jego gry; tura zaczyna się
  dopiero, gdy każdy gracz ją wczyta, a jego gra zostanie sprawdzona. Gracz, który nadal się różni,
  jest wczytywany jeszcze raz. Wkład yodha.

## BetterGameExperience 0.12.0

W grze wieloosobowej wszyscy ludzcy gracze muszą mieć 0.12.0, aby działały poprawki
**[wszyscy gracze]**: poczekalnia, w której są 0.11.0 i 0.12.0, je wyłącza.

### Poprawki gry wieloosobowej [wszyscy gracze]
- **Desynchronizacja po ponownym dołączeniu**: gracz, który dołączył ponownie albo został wczytany
  ponownie sam, prowadził świeżo wczytaną grę obok trwających gier pozostałych. Różnią się one
  danymi, których sprawdzanie synchronizacji nie obejmuje, więc gra znów się rozjeżdżała i wczytywała
  tego gracza raz za razem. Teraz, gdy gra jednego gracza przestaje się zgadzać, host podaje powód na
  czacie i wszyscy od razu wczytują grę hosta; host czeka na sprawdzenie wczytanej gry przez każdego
  gracza, a potem tura zaczyna się dla wszystkich. Wkład yodha.
- **Ponowne dołączenie pod koniec tury**: gracz, który dołączył ponownie w trakcie końca tury, był
  porównywany z następną turą, co powodowało zbędną naprawę, a potem ponowne wczytanie co dwie tury.
  Wkład yodha.
- **Remap Multiplayer Helper**: po remapie host prowadził świeżo wygenerowaną grę, a wszyscy
  pozostali gracze tę samą grę wczytaną z zapisu hosta, i SI podejmowała po obu stronach nieco inne
  decyzje (naprawy po kilka MB na turę w grze 8 graczy). Teraz host także wczytuje własny zapis, więc
  wszystkie komputery prowadzą tę samą grę. Koszt: jeden dodatkowy ekran wczytywania dla hosta, raz
  na remap. Wkład yodha.
- **Każda gra zaczyna się od zera**: niektóre liczniki gry przechodziły z jednej gry do następnej w
  tej samej sesji, przez co obiekty były numerowane inaczej niż u pozostałych graczy. Wkład yodha.

### Gra wieloosobowa
- **Podwójne wczytanie przy uruchamianiu zapisu**: host nie wysyła już drugiej kopii gry graczom,
  którzy jeszcze wczytują, przez co niektórzy wczytywali dwa razy. Gdy kopia nadal zostanie
  odrzucona, czat pokazuje „Zapobieżono podwójnemu wczytaniu”. Wkład yodha.
- **Uszkodzone wiadomości sieciowe**: wiadomość, która wyczyściłaby ustawienia gry, jest teraz też
  odrzucana. Czat podaje gracza i to, czemu zapobieżono („Zapobieżono zawieszeniu gry”,
  „Zapobieżono wyczyszczeniu ustawień gry”). Wkład yodha.
- **Wiadomości ucięte pod koniec wczytywania**: wiadomość wstrzymana w trakcie wczytywania nie jest
  już doklejana do wiadomości, która jeszcze przychodzi w częściach. Wkład yodha.
- **Poprawki BetterGameExperience ustalane przy wczytywaniu**: decyzja o włączeniu poprawek
  **[wszyscy gracze]** jest ponownie sprawdzana przy wczytywaniu i przy zamknięciu ekranu
  wczytywania, nowy host po migracji hosta ją przejmuje (gracze dołączający po migracji grali bez
  nich), a ignorowani gracze pozostają ignorowani po ponownym wczytaniu. Wkład yodha.
- **Nieaktualna kopia gry**: gracz, którego gra nie jest aktualną wersją ze Steama, jest ogłaszany
  na czacie po dołączeniu, bo poprawki sieciowe BetterGameExperience nie mogą działać na jego PC.
- **Okno „Gra wstrzymana”**: nie piętrzy się już podczas wczytywania i nie pojawia się nad ekranem
  wczytywania.
- **Ustawienia gry**: sygnałów drużynowych nie można już włączyć ani wyłączyć po rozpoczęciu gry
  (zmiana nic nie dawała albo zostawiała sygnały aktywne), także we wczytanych zapisach. No
  First/Last moves i sygnałów drużynowych nie można już włączyć w poczekalni, w której któryś gracz
  nie ma BetterGameExperience lub właściwej wersji, ale nadal można je wyłączyć. Wkład mariusbw.

### No First/Last moves
- Opcja pozostaje włączona po włączeniu. Host nie czeka już na gracza, który dwie tury z rzędu nie
  był gotowy, i mówi o tym na czacie. Gracz, który przez 75 s nie dostaje od hosta sygnału startu,
  zaczyna turę sam. Wkład yodha.
- **Spokojniejszy czat**: komunikat o włączeniu skrócono do „No First/Last moves włączone.”, a
  wiersze o wstrzymaniu rozkazów i o starcie pojawiają się raz na grę zamiast co turę. Ogłoszenie,
  że BetterGameExperience jest aktywny, jest wysyłane raz na poczekalnię, a nie przy każdym ponownym
  otwarciu ekranu poczekalni.

### Szybsze wczytywanie
- Nowe ustawienia w karcie BetterGameExperience, wszystkie domyślnie włączone, które skracają
  wczytywanie bez zmiany gry: szybsze wyszukiwanie plików graficznych (kilka sekund z menu
  głównego), baza danych rozgrywki i teksty modów zachowywane między wczytaniami, gdy mody są te same
  (po około 1,5 s), brak czekania na VSync na ekranach wczytywania (około 2 s, tylko przy włączonym
  VSync), jedno czyszczenie Lua przy zamykaniu interfejsu (około 2 s przed remapem), lżejsze
  pierwsze wczytanie dla hosta przy remapie (około 10 s) i szybsza analiza wąskich przejść przy
  generowaniu mapy (około 2 s, ten sam wynik). Dziennik wąskich przejść SI (AI_ChokePoint.csv,
  12 MB) nie jest już domyślnie zapisywany.
- **Wątki zadań**: bezczynne wątki marnują mniej czasu procesora podczas czekania (ustawienia w tej
  samej karcie).

### Interfejs
- **Najwyższa formacja jako pierwsza**: na liście produkcji główny przycisk jednostki szkoli
  najwyższą formację, jaką miasto może wyszkolić (armię, a jeśli nie, korpus; na morzu armadę, a
  jeśli nie, flotę), a pojedyncza jednostka trafia do rozwijanego menu pod spodem. Zakupy za złoto i
  wiarę się nie zmieniają. Domyślnie włączone. Wkład huchenlei.
- **Wczesne ostrzeżenie o obszarach mieszkalnych**: banery twoich miast pokazują pomarańczowy dom,
  gdy miastu brakuje jednego obszaru mieszkalnego do limitu (wzrost -50 %), i czerwony dom, gdy tylko
  go osiągnie (-75 %); gra pokazywała czerwony dom dopiero powyżej limitu. Domyślnie włączone. Wkład
  huchenlei.
- **Karty ustrojowe**: ich efekty nie znikają już do końca gry, gdy ekran rządu zostanie po raz
  pierwszy narysowany w niefortunnym momencie (widziane po ponownym wczytaniu gry); wracały dopiero
  po kolejnym wczytaniu.
- **Flagi jednostek**: jednostki bojowe z ładunkami (legion rzymski, toa, nau…) znów pokazują swoje
  awanse zamiast ładunków; budowniczowie, inżynierowie wojskowi i inne jednostki cywilne i wsparcia
  nadal pokazują swoje ładunki.

### Zgłoszenie błędu
- Dzienniki zawierające tokeny logowania (FiraxisLive.log, EOS.log) są pomijane, a długie
  dzienniki zachowują początek i koniec. Wkład yodha.

## BetterGameExperience 0.11.0

W grze wieloosobowej wszyscy ludzcy gracze muszą mieć 0.11.0, aby działały poprawki
**[wszyscy gracze]**: poczekalnia, w której są 0.10.0 i 0.11.0, je wyłącza.

### Poprawki gry wieloosobowej [wszyscy gracze]
- **Premie do produkcji jednostek, które się zużywają**: usunięcie procentowej premii do produkcji
  jednostek (+30 % do budowniczych z Ilkum albo premii miasta do typu jednostki) odejmowało nieco
  więcej, niż premia dodała, za każdym razem około 0,4 %, a strata rosła przy każdej zamianie karty.
  Usunięcie premii odejmuje teraz dokładnie tyle, ile dodała. Zapisy gry zachowują już narosłą
  stratę. Wkład Serianol.

### Tryb wieloosobowy
- **Gra zawieszona tuż po wczytaniu**: gracz z wolnym połączeniem mógł otrzymać grę od gospodarza
  drugi raz tuż po zakończeniu własnego wczytywania, wczytać ją ponownie z mapy i się zawiesić. Ta
  spóźniona druga kopia jest teraz odrzucana; dołączanie, ponowne łączenie i resynchronizacje
  działają jak wcześniej.
- **Awaria przy wczytywaniu zapisu po innej grze**: ponowne łączenie graczy przekazywanych przez
  gospodarza (0.10.0) mogło odczytać połączenie poprzedniej gry, już zamknięte, i spowodować awarię
  gry podczas wczytywania następnej. Wkład yodha.
- **Ustawienia gry**: opcje gry dodane przez BetterGameExperience są wyszarzone, gdy gospodarz ma
  wyłączone BetterGameExperience.

### Sygnały drużyny
- Nowe grafiki i dźwięki koła, znaczników i każdego wezwania, instalowane z BetterGameExperience.
  Są dodawane do gry tylko wtedy, gdy gospodarz włączy sygnały drużyny, a wszyscy gracze mają
  BetterGameExperience: gracz bez BGE nadal może dołączyć do poczekalni. Wkład mariusbw.
- **Głośność sygnałów drużyny** znajduje się tuż pod efektami dźwiękowymi w opcjach dźwięku, zarówno
  w menu głównym, jak i w grze, a skróty sygnałów pozostają widoczne w opcjach klawiszy. Wkład
  mariusbw.

### Ustawienia
- **Zakładka BetterGameExperience**: włączenie lub wyłączenie BetterGameExperience zawsze prosi o
  ponowne uruchomienie gry. Przywrócenie poprzedniej wartości ustawienia bez ponownego otwierania
  ekranu jest znów zapisywane; wcześniej było ignorowane.

## BetterGameExperience 0.10.0

W grze wieloosobowej wszyscy ludzcy gracze muszą mieć 0.10.0, aby działały poprawki
**[wszyscy gracze]** i sygnały drużyny: poczekalnia, w której są 0.9.x i 0.10.0, je wyłącza.

### Poprawki gry wieloosobowej [wszyscy gracze]
- **Gracze zablokowani na « nie gotowy »**: akcja dyplomatyczna (ambasada, umowa) odebrana tuż przy
  końcu tury mogła sprawić, że host zaczynał następną turę za wcześnie. Niektórzy gracze zaczynali ją
  wtedy już oznaczeni jako skończeni i nie mogli znów być gotowi. Następna tura czeka teraz na
  sprawdzenie przez grę, a akcja nadal zostaje wykonana.
- **Obserwatorzy (Better Spectator Mod)**: przebudowa obserwatorów z 0.9.0 zostaje zawieszona do
  czasu dokładniejszych testów, bo rozgrywki z obserwatorem były mniej stabilne.
  BetterGameExperience nie zmienia już poziomu cywilizacji obserwatorów ani nie zastępuje skryptów
  Better Spectator Mod, a ustawienie « Better Spectator Mod dla obserwatorów » na razie znika.
  Obserwatorzy znów liczą się jako główne cywilizacje dla skryptów, zwycięstw i wyniku, jak przy
  samym Better Spectator Mod. Turystyka i głosowania Kongresu Światowego nadal ich pomijają.
- **Zachowane dla obserwatorów**: zmiana obserwowanego gracza nadal nie powoduje desynchronizacji, a
  tura obserwatora nadal kończy się sama po pauzie MPH. Obie poprawki dotyczą teraz oryginalnych
  skryptów Better Spectator Mod.

### Nowości
- **Sygnały drużyny (BGE)**: zaznacz pole dla swojej drużyny klawiszem **X** albo przytrzymaj **X**,
  by otworzyć koło wezwań (niebezpieczeństwo, już idę, pomoc, wróg zniknął); **Alt+1–4** wysyłają je
  od razu. Członkowie drużyny widzą i słyszą sygnał z nazwą nadawcy, obserwatorzy widzą obie
  drużyny. Domyślnie wyłączone: host zaznacza **Sygnały drużyny (BGE)** w ustawieniach rozgrywki.
  Skróty w opcjach klawiszy gry, głośność w opcjach dźwięku; `/mute nazwa` blokuje czat i sygnały
  gracza. Wkład: mariusbw.

### Tryb wieloosobowy
- **Gracze przekazywani przez hosta**: dwaj gracze, których bezpośrednie połączenie nie powiodło
  się przy dołączaniu, do końca rozgrywki łączą się przez hosta i obaj zostają rozłączeni, gdy host
  wyjdzie. Podczas równoczesnego startu tury BetterGameExperience próbuje teraz połączyć ich znów
  bezpośrednio (najwyżej trzy próby) i informuje o tym na czacie. Wkład: yodha.
- **Pingi zablokowane na 0 ms**: ping gracza na liście graczy nie zamarza już na 0 ms po
  dołączeniu; jest znów mierzony co 5 sekund. Nie trzeba już wychodzić i wracać, by go « naprawić »,
  co często kończyło się desynchronizacją. Wkład: yodha.
- **Uszkodzone wiadomości sieciowe**: ochrona dodana w 0.9.0 działa teraz także z wersją
  DirectX 11 gry. Wkład: yodha.
- **Gracze bez BetterGameExperience**: ostrzeżenie w grze o graczu, który dołączył bez
  BetterGameExperience, nie przerywa się już na błędzie skryptu. Wkład: yodha.

### Instalacja i aktualizacje
- Instalator pobiera pliki i ponownie uruchamia grę bez uruchamiania ukrytych programów
  pomocniczych, a do tego zawiera informacje o wersji: programy antywirusowe powinny rzadziej go
  zgłaszać.

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
