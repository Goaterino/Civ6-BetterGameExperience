# Lista zmian

[English](../CHANGELOG.md) · inne języki: zobacz [listę zmian po angielsku](../CHANGELOG.md).

## BetterGameDLLs 0.5.0: pierwsza wersja publiczna

BetterGameDLLs poprawia i przyspiesza Civilization VI: Gathering Storm za pomocą dwóch bibliotek DLL
umieszczonych obok plików gry. **Żaden plik gry nie jest zastępowany**, więc ani weryfikacja plików
w Steam, ani aktualizacje gry go nie usuwają. Działa z DirectX 11 i DirectX 12, z BBG, Better
Spectator Mod i MPH.

Wszystkie zmiany są wymienione poniżej, według kategorii. Zmiany oznaczone **[wszyscy gracze]**
zmieniają zasady gry: włączają się tylko wtedy, gdy wszyscy ludzcy gracze w poczekalni mają tę samą
wersję BetterGameDLLs. Cała reszta dotyczy tylko twojego komputera. Można bezpiecznie grać z osobami,
które nie mają moda.

### Instalacja i aktualizacje
- Instalator `BetterGameDLLs.exe`: instalacja, aktualizacja, naprawa i odinstalowanie. Znajduje grę
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
- Nowa karta **BetterGameDLLs** w opcjach gry: główny przełącznik, który wyłącza wszystko bez
  odinstalowywania, ustawienie dla każdej funkcji i opcje zaawansowane domyślnie ukryte. Informuje,
  gdy zmiana wymaga ponownego uruchomienia.
- Podziękowania dla autorów wbudowanych modów interfejsu.
- Wszystkie teksty w grze są dostępne w 11 językach Better Balanced Game: angielskim, francuskim,
  niemieckim, hiszpańskim, włoskim, japońskim, koreańskim, polskim, portugalskim (Brazylia), rosyjskim
  i chińskim uproszczonym.

### Tryb wieloosobowy
- **Sprawdzanie w poczekalni**: każdy komputer zgłasza swoją wersję BetterGameDLLs, a host decyduje za
  wszystkich. Wiadomości czatu z przedrostkiem `BGD -` mówią, komu brakuje moda lub kto ma inną
  wersję. Ponowne dołączenia są obsługiwane. Jeśli ktoś dołączy bez moda, gdy poprawki **[wszyscy
  gracze]** są włączone, host dostaje ostrzeżenie.
- Wiadomości czatu `BGD -` są wysyłane po angielsku, aby gracze bez moda mogli je przeczytać. Gracze
  z modem widzą je w języku swojej gry.
- **Opcja gry „No First/Last moves (BGD)”** **[wszyscy gracze]**:
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
Każdy można wyłączyć na karcie BetterGameDLLs. Jeśli masz też moda z Warsztatu, wersja wbudowana w
BGD go zastępuje.
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
  same się wyłączają, dopóki BetterGameDLLs nie zostanie zaktualizowane.
- Gracz, który dołączy w trakcie rozgrywki bez moda, gdy poprawki **[wszyscy gracze]** są włączone,
  ulegnie desynchronizacji. Host dostaje ostrzeżenie.
- Poprawki oznaczone *(eksperymentalne)* nie zostały jeszcze przetestowane w prawdziwej rozgrywce
  albo ich efekt nie został jeszcze w pełni zmierzony.
- Tłumaczenia inne niż angielskie i francuskie nie zostały jeszcze sprawdzone przez rodzimych
  użytkowników języka. Poprawki są mile widziane.
