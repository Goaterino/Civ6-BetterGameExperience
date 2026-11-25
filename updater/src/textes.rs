//! Généré par textes/gen_textes.py depuis textes/<langue>.xml, ne pas modifier à la main.
//! Textes de l'installeur, un par clé `LOC_BGD_INST_*`, dans l'ordre de `LANGUES`.
#![allow(dead_code)]

use crate::lang::Texts;

/// Codes des langues, dans l'ordre de chaque `Texts` (et de `lang::LANGUAGES`).
pub const LANGUES: [&str; 11] = ["en_US", "fr_FR", "de_DE", "es_ES", "it_IT", "ja_JP", "ko_KR", "pl_PL", "pt_BR", "ru_RU", "zh_Hans_CN"];

pub const ADMINISTRATOR_RIGHTS_DENIED: Texts = [
    "administrator rights denied",
    "droits d'administrateur refusés",
    "Administratorrechte verweigert",
    "permisos de administrador denegados",
    "diritti di amministratore negati",
    "管理者権限が拒否されました",
    "관리자 권한이 거부되었습니다",
    "odmowa uprawnień administratora",
    "permissões de administrador negadas",
    "отказано в правах администратора",
    "管理员权限被拒绝",
];

pub const BETTERGAMEDLLS_COULD_NOT_UPDATE: Texts = [
    "BetterGameExperience could not update",
    "BetterGameExperience n'a pas pu se mettre à jour",
    "BetterGameExperience konnte nicht aktualisiert werden",
    "BetterGameExperience no se ha podido actualizar",
    "BetterGameExperience non è riuscito ad aggiornarsi",
    "BetterGameExperience をアップデートできませんでした",
    "BetterGameExperience를 업데이트하지 못했습니다",
    "Nie udało się zaktualizować BetterGameExperience",
    "Não foi possível atualizar o BetterGameExperience",
    "Не удалось обновить BetterGameExperience",
    "BetterGameExperience 无法更新",
];

pub const BETTERGAMEDLLS_IS_NOT_INSTALLED: Texts = [
    "BetterGameExperience is not installed",
    "BetterGameExperience n'est pas installé",
    "BetterGameExperience ist nicht installiert",
    "BetterGameExperience no está instalado",
    "BetterGameExperience non è installato",
    "BetterGameExperience はインストールされていません",
    "BetterGameExperience가 설치되어 있지 않습니다",
    "BetterGameExperience nie jest zainstalowane",
    "O BetterGameExperience não está instalado",
    "BetterGameExperience не установлен",
    "BetterGameExperience 未安装",
];

pub const BETTERGAMEDLLS_IS_UNINSTALLED: Texts = [
    "BetterGameExperience is uninstalled",
    "BetterGameExperience est désinstallé",
    "BetterGameExperience ist deinstalliert",
    "BetterGameExperience está desinstalado",
    "BetterGameExperience è disinstallato",
    "BetterGameExperience はアンインストールされました",
    "BetterGameExperience가 제거되었습니다",
    "BetterGameExperience zostało odinstalowane",
    "O BetterGameExperience foi desinstalado",
    "BetterGameExperience удалён",
    "BetterGameExperience 已卸载",
];

pub const BETTERGAMEDLLS_NEEDS_TO_BE_REPAIRED: Texts = [
    "BetterGameExperience needs to be repaired",
    "BetterGameExperience doit être réparé",
    "BetterGameExperience muss repariert werden",
    "BetterGameExperience necesita repararse",
    "BetterGameExperience deve essere riparato",
    "BetterGameExperience の修復が必要です",
    "BetterGameExperience를 복구해야 합니다",
    "BetterGameExperience wymaga naprawy",
    "O BetterGameExperience precisa ser reparado",
    "BetterGameExperience нуждается в восстановлении",
    "BetterGameExperience 需要修复",
];

pub const BETTERGAMEDLLS_VERSION_X_IS_AVAILABLE: Texts = [
    "BetterGameExperience version {1} is available",
    "La version {1} de BetterGameExperience est disponible",
    "BetterGameExperience-Version {1} ist verfügbar",
    "La versión {1} de BetterGameExperience está disponible",
    "La versione {1} di BetterGameExperience è disponibile",
    "BetterGameExperience のバージョン {1} が利用可能です",
    "BetterGameExperience 버전 {1}을(를) 사용할 수 있습니다",
    "Dostępna jest wersja {1} BetterGameExperience",
    "A versão {1} do BetterGameExperience está disponível",
    "Доступна версия BetterGameExperience {1}",
    "BetterGameExperience 版本 {1} 已可用",
];

pub const BETTERGAMEDLLS_VERSION_X_IS_INSTALLED: Texts = [
    "BetterGameExperience version {1} is installed",
    "La version {1} de BetterGameExperience est installée",
    "BetterGameExperience-Version {1} ist installiert",
    "La versión {1} de BetterGameExperience está instalada",
    "La versione {1} di BetterGameExperience è installata",
    "BetterGameExperience のバージョン {1} がインストールされています",
    "BetterGameExperience 버전 {1}이(가) 설치되어 있습니다",
    "Zainstalowana jest wersja {1} BetterGameExperience",
    "A versão {1} do BetterGameExperience está instalada",
    "Установлена версия BetterGameExperience {1}",
    "已安装 BetterGameExperience 版本 {1}",
];

pub const BETTERGAMEDLLS_X_IS_INSTALLED: Texts = [
    "BetterGameExperience {1} is installed",
    "BetterGameExperience {1} est installé",
    "BetterGameExperience {1} ist installiert",
    "BetterGameExperience {1} está instalado",
    "BetterGameExperience {1} è installato",
    "BetterGameExperience {1} がインストールされました",
    "BetterGameExperience {1}이(가) 설치되었습니다",
    "BetterGameExperience {1} jest zainstalowane",
    "O BetterGameExperience {1} está instalado",
    "BetterGameExperience {1} установлен",
    "BetterGameExperience {1} 已安装",
];

pub const BETTERGAMEDLLS_X_IS_INSTALLED_AND_UP: Texts = [
    "BetterGameExperience {1} is installed and up to date",
    "BetterGameExperience {1} est installé et à jour",
    "BetterGameExperience {1} ist installiert und aktuell",
    "BetterGameExperience {1} está instalado y actualizado",
    "BetterGameExperience {1} è installato e aggiornato",
    "BetterGameExperience {1} はインストール済みで最新です",
    "BetterGameExperience {1}이(가) 설치되어 있으며 최신입니다",
    "BetterGameExperience {1} jest zainstalowane i aktualne",
    "O BetterGameExperience {1} está instalado e atualizado",
    "BetterGameExperience {1} установлен и актуален",
    "BetterGameExperience {1} 已安装且为最新",
];

pub const CHANNEL_PRE_RELEASE_TEST_VERSIONS_AHEAD: Texts = [
    "\nChannel: pre-release (test versions, ahead of their release)",
    "\nCanal : pré-release (versions de test, avant leur publication)",
    "\nKanal: Vorabversion (Testversionen, vor ihrer Veröffentlichung)",
    "\nCanal: versión preliminar (versiones de prueba, antes de su publicación)",
    "\nCanale: versione di prova (versioni di test, prima della loro pubblicazione)",
    "\nチャンネル: プレリリース (公開前のテスト版)",
    "\n채널: 사전 릴리스(공개 전 테스트 버전)",
    "\nKanał: wersje testowe (przed ich publikacją)",
    "\nCanal: versões de teste (antes da publicação)",
    "\nКанал: тестовые версии (до их публикации)",
    "\n渠道：预发布版本（发布前的测试版本）",
];

pub const CIVILIZATION_VI_DID_NOT_CLOSE_CLOSE: Texts = [
    "Civilization VI did not close. Close it, then run the installer again.",
    "Civilization VI ne s'est pas fermé. Ferme-le, puis relance l'installeur.",
    "Civilization VI wurde nicht geschlossen. Schließen Sie es und starten Sie dann das Installationsprogramm erneut.",
    "Civilization VI no se ha cerrado. Ciérralo y vuelve a ejecutar el instalador.",
    "Civilization VI non si è chiuso. Chiudilo, poi avvia di nuovo il programma di installazione.",
    "Civilization VI が終了しませんでした。終了してから、インストーラーを再実行してください。",
    "Civilization VI가 닫히지 않았습니다. 닫은 뒤 설치 프로그램을 다시 실행하세요.",
    "Civilization VI się nie zamknęło. Zamknij je, a następnie uruchom ponownie instalator.",
    "O Civilization VI não fechou. Feche-o e execute o instalador novamente.",
    "Civilization VI не закрылась. Закройте её, затем снова запустите установщик.",
    "Civilization VI 没有关闭。请关闭它，然后重新运行安装程序。",
];

pub const CIVILIZATION_VI_NOT_FOUND: Texts = [
    "Civilization VI not found.",
    "Civilization VI introuvable.",
    "Civilization VI nicht gefunden.",
    "No se ha encontrado Civilization VI.",
    "Civilization VI non trovato.",
    "Civilization VI が見つかりません。",
    "Civilization VI를 찾을 수 없습니다.",
    "Nie znaleziono Civilization VI.",
    "Civilization VI não encontrado.",
    "Civilization VI не найдена.",
    "找不到 Civilization VI。",
];

pub const CIVILIZATION_VI_WAS_NOT_FOUND_IN: Texts = [
    "Civilization VI was not found in the Steam libraries.\n\nRun the installer again with --game-dir \"<game folder>\".",
    "Civilization VI est introuvable dans les bibliothèques Steam.\n\nRelance l'installeur avec --game-dir \"<dossier du jeu>\".",
    "Civilization VI wurde in den Steam-Bibliotheken nicht gefunden.\n\nStarten Sie das Installationsprogramm erneut mit --game-dir \"<Spielordner>\".",
    "No se ha encontrado Civilization VI en las bibliotecas de Steam.\n\nVuelve a ejecutar el instalador con --game-dir \"<carpeta del juego>\".",
    "Civilization VI non è stato trovato nelle librerie di Steam.\n\nAvvia di nuovo il programma di installazione con --game-dir \"<cartella del gioco>\".",
    "Steam ライブラリに Civilization VI が見つかりません。\n\n--game-dir \"<ゲームのフォルダー>\" を付けてインストーラーを再実行してください。",
    "Steam 라이브러리에서 Civilization VI를 찾을 수 없습니다.\n\n--game-dir \"<게임 폴더>\"를 붙여 설치 프로그램을 다시 실행하세요.",
    "Nie znaleziono Civilization VI w bibliotekach Steam.\n\nUruchom ponownie instalator z opcją --game-dir \"<folder gry>\".",
    "O Civilization VI não foi encontrado nas bibliotecas da Steam.\n\nExecute o instalador novamente com --game-dir \"<pasta do jogo>\".",
    "Civilization VI не найдена в библиотеках Steam.\n\nЗапустите установщик снова с параметром --game-dir \"<папка игры>\".",
    "在 Steam 库中找不到 Civilization VI。\n\n请使用 --game-dir \"<游戏文件夹>\" 重新运行安装程序。",
];

pub const CLOSE: Texts = [
    "Close",
    "Fermer",
    "Schließen",
    "Cerrar",
    "Chiudi",
    "閉じる",
    "닫기",
    "Zamknij",
    "Fechar",
    "Закрыть",
    "关闭",
];

pub const CLOSE_CIVILIZATION_VI_THEN_RUN_THE: Texts = [
    "Close Civilization VI, then run the installer again.",
    "Ferme Civilization VI, puis relance l'installeur.",
    "Schließen Sie Civilization VI und starten Sie dann das Installationsprogramm erneut.",
    "Cierra Civilization VI y vuelve a ejecutar el instalador.",
    "Chiudi Civilization VI, poi avvia di nuovo il programma di installazione.",
    "Civilization VI を終了してから、インストーラーを再実行してください。",
    "Civilization VI를 닫은 뒤 설치 프로그램을 다시 실행하세요.",
    "Zamknij Civilization VI, a następnie uruchom ponownie instalator.",
    "Feche o Civilization VI e execute o instalador novamente.",
    "Закройте Civilization VI, затем снова запустите установщик.",
    "请关闭 Civilization VI，然后重新运行安装程序。",
];

pub const COULD_NOT_READ_THE_LATEST_VERSION: Texts = [
    "\n\nCould not read the latest version: {1}",
    "\n\nImpossible de lire la dernière version : {1}",
    "\n\nDie neueste Version konnte nicht gelesen werden: {1}",
    "\n\nNo se ha podido leer la última versión: {1}",
    "\n\nImpossibile leggere l'ultima versione: {1}",
    "\n\n最新バージョンを読み込めませんでした: {1}",
    "\n\n최신 버전을 읽을 수 없습니다: {1}",
    "\n\nNie można odczytać najnowszej wersji: {1}",
    "\n\nNão foi possível ler a versão mais recente: {1}",
    "\n\nНе удалось прочитать последнюю версию: {1}",
    "\n\n无法读取最新版本：{1}",
];

pub const DETAILS: Texts = [
    "Details",
    "Détails",
    "Details",
    "Detalles",
    "Dettagli",
    "詳細",
    "세부 정보",
    "Szczegóły",
    "Detalhes",
    "Подробности",
    "详情",
];

pub const DOWNLOAD_OF_X_X: Texts = [
    "download of {1}: {2}",
    "téléchargement de {1} : {2}",
    "Download von {1}: {2}",
    "descarga de {1}: {2}",
    "download di {1}: {2}",
    "{1} のダウンロード: {2}",
    "{1} 다운로드: {2}",
    "pobieranie {1}: {2}",
    "download de {1}: {2}",
    "загрузка {1}: {2}",
    "下载 {1}：{2}",
];

pub const FAILURE_CODE_X: Texts = [
    "failure (code {1})",
    "échec (code {1})",
    "Fehler (Code {1})",
    "fallo (código {1})",
    "errore (codice {1})",
    "失敗 (コード {1})",
    "실패(코드 {1})",
    "błąd (kod {1})",
    "falha (código {1})",
    "ошибка (код {1})",
    "失败（代码 {1}）",
];

pub const FILE_FROM_ANOTHER_PROGRAM: Texts = [
    "file from another program",
    "fichier d'un autre programme",
    "Datei eines anderen Programms",
    "archivo de otro programa",
    "file di un altro programma",
    "別のプログラムのファイル",
    "다른 프로그램의 파일",
    "plik innego programu",
    "arquivo de outro programa",
    "файл другой программы",
    "其他程序的文件",
];

pub const GAME_X: Texts = [
    "Game: {1}",
    "Jeu : {1}",
    "Spiel: {1}",
    "Juego: {1}",
    "Gioco: {1}",
    "ゲーム: {1}",
    "게임: {1}",
    "Gra: {1}",
    "Jogo: {1}",
    "Игра: {1}",
    "游戏：{1}",
];

pub const GO_BACK_TO_VERSION_X_REPLACES: Texts = [
    "Go back to version {1}\nReplaces the pre-release with the latest release.",
    "Revenir à la version {1}\nRemplace la pré-release par la dernière release.",
    "Zurück zu Version {1}\nErsetzt die Vorabversion durch die neueste Version.",
    "Volver a la versión {1}\nSustituye la versión preliminar por la última versión publicada.",
    "Torna alla versione {1}\nSostituisce la versione di prova con l'ultima versione pubblicata.",
    "バージョン {1} に戻す\nプレリリースを最新のリリースに置き換えます。",
    "버전 {1}(으)로 돌아가기\n사전 릴리스를 최신 릴리스로 바꿉니다.",
    "Wróć do wersji {1}\nZastępuje wersję testową najnowszą wersją.",
    "Voltar para a versão {1}\nSubstitui a versão de teste pela versão publicada mais recente.",
    "Вернуться к версии {1}\nЗаменяет тестовую версию последним выпуском.",
    "回到版本 {1}\n用最新的正式版本替换预发布版本。",
];

pub const HIDE_DETAILS: Texts = [
    "Hide details",
    "Masquer les détails",
    "Details ausblenden",
    "Ocultar detalles",
    "Nascondi dettagli",
    "詳細を隠す",
    "세부 정보 숨기기",
    "Ukryj szczegóły",
    "Ocultar detalhes",
    "Скрыть подробности",
    "隐藏详情",
];

pub const INSTALLING_BETTERGAMEDLLS_X: Texts = [
    "Installing BetterGameExperience {1}…",
    "Installation de BetterGameExperience {1}…",
    "BetterGameExperience {1} wird installiert …",
    "Instalando BetterGameExperience {1}…",
    "Installazione di BetterGameExperience {1}…",
    "BetterGameExperience {1} をインストール中…",
    "BetterGameExperience {1}을(를) 설치하는 중…",
    "Instalowanie BetterGameExperience {1}…",
    "Instalando o BetterGameExperience {1}…",
    "Установка BetterGameExperience {1}…",
    "正在安装 BetterGameExperience {1}…",
];

pub const INSTALL_DOWNLOADS_AND_INSTALLS_VERSION_X: Texts = [
    "Install\nDownloads and installs version {1}.",
    "Installer\nTélécharge et installe la version {1}.",
    "Installieren\nLädt Version {1} herunter und installiert sie.",
    "Instalar\nDescarga e instala la versión {1}.",
    "Installa\nScarica e installa la versione {1}.",
    "インストール\nバージョン {1} をダウンロードしてインストールします。",
    "설치\n버전 {1}을(를) 내려받아 설치합니다.",
    "Zainstaluj\nPobiera i instaluje wersję {1}.",
    "Instalar\nBaixa e instala a versão {1}.",
    "Установить\nЗагружает и устанавливает версию {1}.",
    "安装\n下载并安装版本 {1}。",
];

pub const INVALID_MANIFEST_SIGNATURE: Texts = [
    "invalid manifest signature",
    "signature du manifeste invalide",
    "Signatur des Manifests ungültig",
    "firma del manifiesto no válida",
    "firma del manifesto non valida",
    "マニフェストの署名が無効です",
    "매니페스트 서명이 올바르지 않습니다",
    "nieprawidłowy podpis manifestu",
    "assinatura do manifesto inválida",
    "недействительная подпись манифеста",
    "清单签名无效",
];

pub const LATEST_VERSION_UNKNOWN: Texts = [
    "Latest version unknown",
    "Dernière version inconnue",
    "Neueste Version unbekannt",
    "Última versión desconocida",
    "Ultima versione sconosciuta",
    "最新バージョン不明",
    "최신 버전 알 수 없음",
    "Najnowsza wersja nieznana",
    "Versão mais recente desconhecida",
    "Последняя версия неизвестна",
    "最新版本未知",
];

pub const LINUX_CHOOSE_A_NUMBER: Texts = [
    "Choose a number (empty to quit):",
    "Choisis un numéro (vide pour quitter) :",
    "Gib eine Nummer ein (leer zum Beenden):",
    "Elige un número (vacío para salir):",
    "Scegli un numero (vuoto per uscire):",
    "番号を選んでください（空で終了）:",
    "번호를 선택하세요(비우면 종료):",
    "Wybierz numer (puste, aby wyjść):",
    "Escolha um número (vazio para sair):",
    "Выберите номер (пусто — выход):",
    "请选择编号（留空退出）:",
];

pub const LINUX_FOLDER_NOT_WRITABLE_X: Texts = [
    "Cannot write to {1}.\n\nCheck that the folder is not on a read-only drive, is not owned by root, and that the drive's permissions (NTFS, for example) allow writing.",
    "Impossible d'écrire dans {1}.\n\nVérifie que le dossier n'est pas sur un disque en lecture seule, n'appartient pas à root, et que les permissions du disque (NTFS, par exemple) autorisent l'écriture.",
    "In {1} kann nicht geschrieben werden.\n\nPrüfe, dass der Ordner nicht auf einem schreibgeschützten Laufwerk liegt, nicht root gehört und dass die Berechtigungen des Laufwerks (z. B. NTFS) Schreiben erlauben.",
    "No se puede escribir en {1}.\n\nComprueba que la carpeta no está en un disco de solo lectura, que no pertenece a root y que los permisos del disco (NTFS, por ejemplo) permiten escribir.",
    "Impossibile scrivere in {1}.\n\nControlla che la cartella non sia su un disco di sola lettura, non appartenga a root e che i permessi del disco (NTFS, ad esempio) consentano la scrittura.",
    "{1} に書き込めません。\n\nフォルダーが読み取り専用のドライブにないこと、root の所有になっていないこと、ドライブの権限（たとえば NTFS）で書き込みが許可されていることを確認してください。",
    "{1}에 쓸 수 없습니다.\n\n폴더가 읽기 전용 드라이브에 있지 않은지, root 소유가 아닌지, 드라이브의 권한(예: NTFS)이 쓰기를 허용하는지 확인하세요.",
    "Nie można zapisać w {1}.\n\nSprawdź, czy folder nie jest na dysku tylko do odczytu, nie należy do roota i czy uprawnienia dysku (np. NTFS) pozwalają na zapis.",
    "Não foi possível gravar em {1}.\n\nVerifique se a pasta não está em um disco somente leitura, se não pertence ao root e se as permissões do disco (NTFS, por exemplo) permitem gravação.",
    "Не удаётся записать в {1}.\n\nПроверьте, что папка не находится на диске только для чтения, не принадлежит root и что права диска (например, NTFS) разрешают запись.",
    "无法写入 {1}。\n\n请检查该文件夹不在只读磁盘上、不属于 root，并且磁盘权限（例如 NTFS）允许写入。",
];

pub const LINUX_GAME_NOT_FOUND_X: Texts = [
    "Civilization VI was not found in the Steam libraries:\n{1}\n\nThe game must be the Windows version run through Steam Play (Proton). A library marked (?) could not be read: check that its drive is mounted. Otherwise run again with --game-dir \"<game folder>\".",
    "Civilization VI est introuvable dans les bibliothèques Steam :\n{1}\n\nLe jeu doit être la version Windows, lancée par Steam Play (Proton). Une bibliothèque marquée (?) n'a pas pu être lue : vérifie que son disque est monté. Sinon relance avec --game-dir \"<dossier du jeu>\".",
    "Civilization VI wurde in den Steam-Bibliotheken nicht gefunden:\n{1}\n\nDas Spiel muss die Windows-Version sein, die über Steam Play (Proton) läuft. Eine mit (?) markierte Bibliothek konnte nicht gelesen werden: Prüfe, ob ihr Laufwerk eingebunden ist. Andernfalls starte erneut mit --game-dir \"<Spielordner>\".",
    "Civilization VI no se ha encontrado en las bibliotecas de Steam:\n{1}\n\nEl juego debe ser la versión de Windows, ejecutada con Steam Play (Proton). Una biblioteca marcada con (?) no se ha podido leer: comprueba que su disco está montado. Si no, vuelve a ejecutar con --game-dir \"<carpeta del juego>\".",
    "Civilization VI non è stato trovato nelle librerie di Steam:\n{1}\n\nIl gioco deve essere la versione Windows, eseguita con Steam Play (Proton). Una libreria contrassegnata con (?) non è stata letta: controlla che il suo disco sia montato. Altrimenti riesegui con --game-dir \"<cartella del gioco>\".",
    "Steam ライブラリに Civilization VI が見つかりません:\n{1}\n\nゲームは Steam Play（Proton）で実行する Windows 版である必要があります。(?) が付いたライブラリは読み取れませんでした。そのドライブがマウントされているか確認してください。それでも見つからない場合は --game-dir \"<ゲームのフォルダー>\" を付けて再実行してください。",
    "Steam 라이브러리에서 Civilization VI를 찾을 수 없습니다:\n{1}\n\n게임은 Steam Play(Proton)로 실행하는 Windows 버전이어야 합니다. (?)로 표시된 라이브러리는 읽을 수 없었습니다. 해당 드라이브가 마운트되어 있는지 확인하세요. 그래도 안 되면 --game-dir \"<게임 폴더>\"를 붙여 다시 실행하세요.",
    "Nie znaleziono Civilization VI w bibliotekach Steam:\n{1}\n\nGra musi być wersją Windows uruchamianą przez Steam Play (Proton). Biblioteki oznaczonej (?) nie udało się odczytać: sprawdź, czy jej dysk jest zamontowany. W przeciwnym razie uruchom ponownie z --game-dir \"<folder gry>\".",
    "Civilization VI não foi encontrado nas bibliotecas do Steam:\n{1}\n\nO jogo deve ser a versão Windows executada pelo Steam Play (Proton). Uma biblioteca marcada com (?) não pôde ser lida: verifique se o disco dela está montado. Caso contrário, execute novamente com --game-dir \"<pasta do jogo>\".",
    "Civilization VI не найдена в библиотеках Steam:\n{1}\n\nИгра должна быть версией для Windows, запущенной через Steam Play (Proton). Библиотеку с пометкой (?) прочитать не удалось: проверьте, что её диск подключён. Иначе запустите снова с --game-dir \"<папка игры>\".",
    "在 Steam 库中找不到 Civilization VI:\n{1}\n\n游戏必须是通过 Steam Play（Proton）运行的 Windows 版。标有 (?) 的库无法读取：请检查其磁盘是否已挂载。否则请使用 --game-dir \"<游戏文件夹>\" 重新运行。",
];

pub const LINUX_LAUNCH_OPTION_MISSING_X: Texts = [
    "Civilization VI's launch options in Steam do not contain {1} before %command%: without it, BetterGameExperience does not load. If you have just set it, Steam may not have saved it yet (it saves it when it closes).",
    "Les options de lancement de Civilization VI dans Steam ne contiennent pas {1} avant %command% : sans elle, BetterGameExperience ne se charge pas. Si tu viens de la régler, Steam ne l'a peut-être pas encore enregistrée (il le fait en se fermant).",
    "Die Startoptionen von Civilization VI in Steam enthalten {1} nicht vor %command%: Ohne diese Option wird BetterGameExperience nicht geladen. Wenn du sie gerade erst gesetzt hast, hat Steam sie vielleicht noch nicht gespeichert (das geschieht beim Beenden von Steam).",
    "Las opciones de lanzamiento de Civilization VI en Steam no contienen {1} antes de %command%: sin ella, BetterGameExperience no se carga. Si acabas de configurarla, puede que Steam aún no la haya guardado (lo hace al cerrarse).",
    "Le opzioni di avvio di Civilization VI in Steam non contengono {1} prima di %command%: senza di essa BetterGameExperience non si carica. Se l'hai appena impostata, Steam potrebbe non averla ancora salvata (lo fa alla chiusura).",
    "Steam の Civilization VI の起動オプションで、%command% の前に {1} がありません。これがないと BetterGameExperience は読み込まれません。設定したばかりの場合は、Steam がまだ保存していない可能性があります（Steam の終了時に保存されます）。",
    "Steam의 Civilization VI 실행 옵션에서 %command% 앞에 {1}이(가) 없습니다. 이 옵션이 없으면 BetterGameExperience가 로드되지 않습니다. 방금 설정했다면 Steam이 아직 저장하지 않았을 수 있습니다(Steam을 종료할 때 저장됩니다).",
    "Opcje uruchamiania Civilization VI w Steam nie zawierają {1} przed %command%: bez tego BetterGameExperience się nie wczyta. Jeśli opcja została właśnie ustawiona, Steam mógł jej jeszcze nie zapisać (robi to przy zamykaniu).",
    "As opções de inicialização do Civilization VI no Steam não contêm {1} antes de %command%: sem isso, o BetterGameExperience não é carregado. Se você acabou de defini-la, o Steam pode ainda não tê-la salvado (ele salva ao fechar).",
    "В параметрах запуска Civilization VI в Steam нет {1} перед %command%: без этого BetterGameExperience не загружается. Если вы только что их задали, Steam мог ещё не сохранить изменения (он сохраняет их при закрытии).",
    "Steam 中 Civilization VI 的启动选项在 %command% 之前没有 {1}：没有它，BetterGameExperience 不会加载。如果你刚刚设置，Steam 可能还没有保存（它在关闭时保存）。",
];

pub const LINUX_LAUNCH_OPTION_STEPS_X: Texts = [
    "To load BetterGameExperience in Proton, set the launch options in Steam yourself:\n\n1. In your Steam library, right-click Civilization VI, then Properties.\n2. In General, type this in Launch Options:\n   {1}\n3. In Compatibility, check that a Proton version is forced for the game (BetterGameExperience needs the Windows version).\n\nIf Launch Options already has text, add WINEDLLOVERRIDES=\"winmm=n,b\" at the very start and keep %command%.",
    "Pour charger BetterGameExperience dans Proton, règle toi-même les options de lancement dans Steam :\n\n1. Dans ta bibliothèque Steam, clic droit sur Civilization VI, puis Propriétés.\n2. Dans Général, saisis ceci dans Options de lancement :\n   {1}\n3. Dans Compatibilité, vérifie qu'une version de Proton est imposée pour le jeu (BetterGameExperience a besoin de la version Windows).\n\nSi les options de lancement contiennent déjà du texte, ajoute WINEDLLOVERRIDES=\"winmm=n,b\" tout au début et garde %command%.",
    "Damit BetterGameExperience in Proton geladen wird, setze die Startoptionen in Steam selbst:\n\n1. Klicke in deiner Steam-Bibliothek mit der rechten Maustaste auf Civilization VI und wähle Eigenschaften.\n2. Gib unter Allgemein bei Startoptionen Folgendes ein:\n   {1}\n3. Prüfe unter Kompatibilität, dass für das Spiel eine Proton-Version erzwungen wird (BetterGameExperience benötigt die Windows-Version).\n\nStehen in den Startoptionen schon Einträge, füge WINEDLLOVERRIDES=\"winmm=n,b\" ganz am Anfang hinzu und behalte %command%.",
    "Para cargar BetterGameExperience en Proton, configura tú mismo las opciones de lanzamiento en Steam:\n\n1. En tu biblioteca de Steam, haz clic derecho en Civilization VI y elige Propiedades.\n2. En General, escribe esto en Opciones de lanzamiento:\n   {1}\n3. En Compatibilidad, comprueba que se fuerza una versión de Proton para el juego (BetterGameExperience necesita la versión de Windows).\n\nSi las opciones de lanzamiento ya tienen texto, añade WINEDLLOVERRIDES=\"winmm=n,b\" al principio y conserva %command%.",
    "Per caricare BetterGameExperience in Proton, imposta tu stesso le opzioni di avvio in Steam:\n\n1. Nella tua libreria Steam, clic destro su Civilization VI, poi Proprietà.\n2. In Generale, scrivi questo in Opzioni di avvio:\n   {1}\n3. In Compatibilità, controlla che sia forzata una versione di Proton per il gioco (BetterGameExperience richiede la versione Windows).\n\nSe le opzioni di avvio contengono già del testo, aggiungi WINEDLLOVERRIDES=\"winmm=n,b\" all'inizio e mantieni %command%.",
    "BetterGameExperience を Proton で読み込むには、Steam の起動オプションを自分で設定してください:\n\n1. Steam ライブラリで Civilization VI を右クリックし、プロパティを選びます。\n2. 「一般」の起動オプションに次を入力します:\n   {1}\n3. 「互換性」で、このゲームに Proton のバージョンが強制されていることを確認します（BetterGameExperience には Windows 版が必要です）。\n\n起動オプションにすでに文字列がある場合は、先頭に WINEDLLOVERRIDES=\"winmm=n,b\" を追加し、%command% はそのまま残してください。",
    "BetterGameExperience를 Proton에서 불러오려면 Steam 실행 옵션을 직접 설정하세요:\n\n1. Steam 라이브러리에서 Civilization VI를 마우스 오른쪽 버튼으로 클릭한 뒤 속성을 선택합니다.\n2. 일반의 실행 옵션에 다음을 입력합니다:\n   {1}\n3. 호환성에서 이 게임에 Proton 버전이 강제로 지정되어 있는지 확인합니다(BetterGameExperience에는 Windows 버전이 필요합니다).\n\n실행 옵션에 이미 내용이 있으면 맨 앞에 WINEDLLOVERRIDES=\"winmm=n,b\"를 추가하고 %command%는 그대로 두세요.",
    "Aby wczytać BetterGameExperience w Protonie, ustaw samodzielnie opcje uruchamiania w Steam:\n\n1. W bibliotece Steam kliknij prawym przyciskiem Civilization VI i wybierz Właściwości.\n2. W Ogólne wpisz w Opcjach uruchamiania:\n   {1}\n3. W Zgodności sprawdź, czy dla gry wymuszona jest wersja Protona (BetterGameExperience wymaga wersji Windows).\n\nJeśli w opcjach uruchamiania jest już tekst, dopisz WINEDLLOVERRIDES=\"winmm=n,b\" na samym początku i zachowaj %command%.",
    "Para carregar o BetterGameExperience no Proton, defina você mesmo as opções de inicialização no Steam:\n\n1. Na sua biblioteca Steam, clique com o botão direito em Civilization VI e escolha Propriedades.\n2. Em Geral, digite isto em Opções de inicialização:\n   {1}\n3. Em Compatibilidade, verifique se uma versão do Proton está forçada para o jogo (o BetterGameExperience precisa da versão Windows).\n\nSe as opções de inicialização já tiverem texto, adicione WINEDLLOVERRIDES=\"winmm=n,b\" no começo e mantenha %command%.",
    "Чтобы BetterGameExperience загружался в Proton, задайте параметры запуска в Steam самостоятельно:\n\n1. В библиотеке Steam щёлкните Civilization VI правой кнопкой мыши и выберите «Свойства».\n2. На вкладке «Общие» введите в «Параметры запуска»:\n   {1}\n3. На вкладке «Совместимость» убедитесь, что для игры принудительно выбрана версия Proton (BetterGameExperience требуется версия для Windows).\n\nЕсли в параметрах запуска уже есть текст, добавьте WINEDLLOVERRIDES=\"winmm=n,b\" в самое начало и оставьте %command%.",
    "要让 BetterGameExperience 在 Proton 中加载，请自行在 Steam 中设置启动选项:\n\n1. 在 Steam 库中右键单击 Civilization VI，然后选择“属性”。\n2. 在“通用”的“启动选项”中输入:\n   {1}\n3. 在“兼容性”中，检查该游戏已强制使用某个 Proton 版本（BetterGameExperience 需要 Windows 版）。\n\n如果启动选项中已有内容，请在最开头添加 WINEDLLOVERRIDES=\"winmm=n,b\"，并保留 %command%。",
];

pub const LINUX_RUN_FROM_A_TERMINAL: Texts = [
    "Run BetterGameExperience from a terminal.\n\nCommands: status, install, uninstall, launch-option, channel release|prerelease, lang <code>\nOptions: --game-dir <folder>, --yes",
    "Lance BetterGameExperience depuis un terminal.\n\nCommandes : status, install, uninstall, launch-option, channel release|prerelease, lang <code>\nOptions : --game-dir <dossier>, --yes",
    "Starte BetterGameExperience in einem Terminal.\n\nBefehle: status, install, uninstall, launch-option, channel release|prerelease, lang <Code>\nOptionen: --game-dir <Ordner>, --yes",
    "Ejecuta BetterGameExperience desde un terminal.\n\nComandos: status, install, uninstall, launch-option, channel release|prerelease, lang <código>\nOpciones: --game-dir <carpeta>, --yes",
    "Esegui BetterGameExperience da un terminale.\n\nComandi: status, install, uninstall, launch-option, channel release|prerelease, lang <codice>\nOpzioni: --game-dir <cartella>, --yes",
    "BetterGameExperience をターミナルから実行してください。\n\nコマンド: status, install, uninstall, launch-option, channel release|prerelease, lang <コード>\nオプション: --game-dir <フォルダー>, --yes",
    "BetterGameExperience를 터미널에서 실행하세요.\n\n명령: status, install, uninstall, launch-option, channel release|prerelease, lang <코드>\n옵션: --game-dir <폴더>, --yes",
    "Uruchom BetterGameExperience z terminala.\n\nPolecenia: status, install, uninstall, launch-option, channel release|prerelease, lang <kod>\nOpcje: --game-dir <folder>, --yes",
    "Execute o BetterGameExperience em um terminal.\n\nComandos: status, install, uninstall, launch-option, channel release|prerelease, lang <código>\nOpções: --game-dir <pasta>, --yes",
    "Запустите BetterGameExperience из терминала.\n\nКоманды: status, install, uninstall, launch-option, channel release|prerelease, lang <код>\nПараметры: --game-dir <папка>, --yes",
    "请在终端中运行 BetterGameExperience。\n\n命令: status, install, uninstall, launch-option, channel release|prerelease, lang <代码>\n选项: --game-dir <文件夹>, --yes",
];

pub const LINUX_SHOW_THE_LAUNCH_OPTION: Texts = [
    "Show the Steam launch option to set",
    "Afficher l'option de lancement Steam à régler",
    "Die einzutragende Steam-Startoption anzeigen",
    "Mostrar la opción de lanzamiento de Steam que hay que configurar",
    "Mostra l'opzione di avvio di Steam da impostare",
    "設定する Steam の起動オプションを表示",
    "설정해야 할 Steam 실행 옵션 표시",
    "Pokaż opcję uruchamiania Steam do ustawienia",
    "Mostrar a opção de inicialização do Steam a definir",
    "Показать параметр запуска Steam, который нужно задать",
    "显示需要设置的 Steam 启动选项",
];

pub const LOOKING_FOR_THE_LATEST_VERSION: Texts = [
    "Looking for the latest version…",
    "Recherche de la dernière version…",
    "Suche nach der neuesten Version …",
    "Buscando la última versión…",
    "Ricerca dell'ultima versione…",
    "最新バージョンを確認中…",
    "최신 버전을 확인하는 중…",
    "Szukanie najnowszej wersji…",
    "Procurando a versão mais recente…",
    "Поиск последней версии…",
    "正在查找最新版本…",
];

pub const MAC_CONFIGURE_STEAM: Texts = [
    "Configure Steam\nAdds the BetterGameExperience launch option to Civilization VI. Steam restarts if it is open.",
    "Configurer Steam\nAjoute l'option de lancement de BetterGameExperience à Civilization VI. Steam redémarre s'il est ouvert.",
    "Steam einrichten\nFügt die Startoption von BetterGameExperience zu Civilization VI hinzu. Steam startet neu, falls es geöffnet ist.",
    "Configurar Steam\nAñade la opción de lanzamiento de BetterGameExperience a Civilization VI. Steam se reinicia si está abierto.",
    "Configura Steam\nAggiunge l'opzione di avvio di BetterGameExperience a Civilization VI. Steam si riavvia se è aperto.",
    "Steam を設定\nCivilization VI に BetterGameExperience の起動オプションを追加します。Steam が開いている場合は再起動します。",
    "Steam 설정\nCivilization VI에 BetterGameExperience 실행 옵션을 추가합니다. Steam이 열려 있으면 다시 시작합니다.",
    "Skonfiguruj Steam\nDodaje opcję uruchamiania BetterGameExperience do Civilization VI. Steam uruchomi się ponownie, jeśli jest otwarty.",
    "Configurar a Steam\nAdiciona a opção de inicialização do BetterGameExperience ao Civilization VI. A Steam reinicia se estiver aberta.",
    "Настроить Steam\nДобавляет параметр запуска BetterGameExperience к Civilization VI. Если Steam открыт, он перезапустится.",
    "配置 Steam\n为 Civilization VI 添加 BetterGameExperience 的启动选项。如果 Steam 已打开，它会重新启动。",
];

pub const MAC_CONFIGURING_STEAM: Texts = [
    "Configuring Steam",
    "Configuration de Steam",
    "Steam wird eingerichtet",
    "Configurando Steam",
    "Configurazione di Steam",
    "Steam を設定しています",
    "Steam 설정 중",
    "Konfigurowanie Steam",
    "Configurando a Steam",
    "Настройка Steam",
    "正在配置 Steam",
];

pub const MAC_LAUNCH_OPTION_IN_PLACE: Texts = [
    "\nSteam launch option: in place",
    "\nOption de lancement Steam : en place",
    "\nSteam-Startoption: eingerichtet",
    "\nOpción de lanzamiento de Steam: configurada",
    "\nOpzione di avvio di Steam: impostata",
    "\nSteam の起動オプション：設定済み",
    "\nSteam 실행 옵션: 설정됨",
    "\nOpcja uruchamiania Steam: ustawiona",
    "\nOpção de inicialização da Steam: configurada",
    "\nПараметр запуска Steam: задан",
    "\nSteam 启动选项：已设置",
];

pub const MAC_LAUNCH_OPTION_MISSING: Texts = [
    "\n\nOn macOS, BetterGameExperience is only active when Steam starts the game through this launcher: Configure Steam adds its launch option to Civilization VI.",
    "\n\nSous macOS, BetterGameExperience n'est actif que si Steam lance le jeu par ce lanceur : Configurer Steam ajoute son option de lancement à Civilization VI.",
    "\n\nUnter macOS ist BetterGameExperience nur aktiv, wenn Steam das Spiel über diesen Launcher startet: Steam einrichten fügt seine Startoption zu Civilization VI hinzu.",
    "\n\nEn macOS, BetterGameExperience solo está activo si Steam inicia el juego con este lanzador: Configurar Steam añade su opción de lanzamiento a Civilization VI.",
    "\n\nSu macOS, BetterGameExperience è attivo solo se Steam avvia il gioco tramite questo launcher: Configura Steam aggiunge la sua opzione di avvio a Civilization VI.",
    "\n\nmacOS では、Steam がこのランチャー経由でゲームを起動したときだけ BetterGameExperience が有効になります。「Steam を設定」で Civilization VI に起動オプションを追加します。",
    "\n\nmacOS에서는 Steam이 이 런처를 통해 게임을 실행할 때만 BetterGameExperience가 작동합니다. Steam 설정을 누르면 Civilization VI에 실행 옵션이 추가됩니다.",
    "\n\nW macOS BetterGameExperience działa tylko wtedy, gdy Steam uruchamia grę przez ten launcher: Skonfiguruj Steam dodaje jego opcję uruchamiania do Civilization VI.",
    "\n\nNo macOS, o BetterGameExperience só fica ativo quando a Steam inicia o jogo por este iniciador: Configurar a Steam adiciona a opção de inicialização dele ao Civilization VI.",
    "\n\nВ macOS BetterGameExperience работает, только если Steam запускает игру через этот лаунчер: «Настроить Steam» добавит его параметр запуска к Civilization VI.",
    "\n\n在 macOS 上，只有当 Steam 通过此启动器启动游戏时，BetterGameExperience 才会生效：“配置 Steam”会为 Civilization VI 添加它的启动选项。",
];

pub const MAC_PASTE_IT_IN_STEAM_X: Texts = [
    "In Steam, right-click Civilization VI > Properties > General, and paste it in Launch options:\n\n{1}\n\nThen start the game from Steam as usual.",
    "Dans Steam, clic droit sur Civilization VI > Propriétés > Général, puis colle-la dans Options de lancement :\n\n{1}\n\nLance ensuite le jeu depuis Steam, comme d'habitude.",
    "Klicke in Steam mit der rechten Maustaste auf Civilization VI > Eigenschaften > Allgemein und füge sie unter Startoptionen ein:\n\n{1}\n\nStarte das Spiel dann wie gewohnt über Steam.",
    "En Steam, haz clic derecho en Civilization VI > Propiedades > General y pégala en Opciones de lanzamiento:\n\n{1}\n\nLuego inicia el juego desde Steam, como siempre.",
    "In Steam, fai clic destro su Civilization VI > Proprietà > Generali e incollala in Opzioni di avvio:\n\n{1}\n\nPoi avvia il gioco da Steam, come al solito.",
    "Steam で Civilization VI を右クリック > プロパティ > 一般 を開き、起動オプションに貼り付けてください：\n\n{1}\n\nその後、いつもどおり Steam からゲームを起動してください。",
    "Steam에서 Civilization VI를 마우스 오른쪽 버튼으로 클릭 > 속성 > 일반으로 이동해 실행 옵션에 붙여 넣으세요:\n\n{1}\n\n그런 다음 평소처럼 Steam에서 게임을 실행하세요.",
    "W Steam kliknij prawym przyciskiem Civilization VI > Właściwości > Ogólne i wklej ją w Opcjach uruchamiania:\n\n{1}\n\nNastępnie uruchom grę ze Steam jak zwykle.",
    "Na Steam, clique com o botão direito em Civilization VI > Propriedades > Geral e cole-a em Opções de inicialização:\n\n{1}\n\nDepois, inicie o jogo pela Steam, como de costume.",
    "В Steam щёлкните правой кнопкой по Civilization VI > Свойства > Общие и вставьте его в «Параметры запуска»:\n\n{1}\n\nЗатем запускайте игру из Steam как обычно.",
    "在 Steam 中右键点击 Civilization VI > 属性 > 通用，然后粘贴到启动选项中：\n\n{1}\n\n之后像往常一样从 Steam 启动游戏。",
];

pub const MAC_RESTART_STEAM: Texts = [
    "Restart Steam\nCloses Steam, adds the launch option, then reopens Steam.",
    "Redémarrer Steam\nFerme Steam, ajoute l'option de lancement, puis rouvre Steam.",
    "Steam neu starten\nSchließt Steam, fügt die Startoption hinzu und öffnet Steam wieder.",
    "Reiniciar Steam\nCierra Steam, añade la opción de lanzamiento y vuelve a abrir Steam.",
    "Riavvia Steam\nChiude Steam, aggiunge l'opzione di avvio, poi riapre Steam.",
    "Steam を再起動\nSteam を終了し、起動オプションを追加してから、Steam を再び開きます。",
    "Steam 다시 시작\nSteam을 종료하고 실행 옵션을 추가한 뒤 Steam을 다시 엽니다.",
    "Uruchom ponownie Steam\nZamyka Steam, dodaje opcję uruchamiania, a potem ponownie otwiera Steam.",
    "Reiniciar a Steam\nFecha a Steam, adiciona a opção de inicialização e abre a Steam de novo.",
    "Перезапустить Steam\nЗакрывает Steam, добавляет параметр запуска и снова открывает Steam.",
    "重新启动 Steam\n关闭 Steam，添加启动选项，然后重新打开 Steam。",
];

pub const MAC_STEAM_DID_NOT_CLOSE: Texts = [
    "Steam did not close.",
    "Steam ne s'est pas fermé.",
    "Steam wurde nicht geschlossen.",
    "Steam no se cerró.",
    "Steam non si è chiuso.",
    "Steam が終了しませんでした。",
    "Steam이 종료되지 않았습니다.",
    "Steam się nie zamknął.",
    "A Steam não fechou.",
    "Steam не закрылся.",
    "Steam 没有关闭。",
];

pub const MAC_STEAM_MUST_RESTART: Texts = [
    "Steam needs to restart",
    "Steam doit redémarrer",
    "Steam muss neu starten",
    "Steam debe reiniciarse",
    "Steam deve riavviarsi",
    "Steam の再起動が必要です",
    "Steam을 다시 시작해야 합니다",
    "Steam musi się uruchomić ponownie",
    "A Steam precisa reiniciar",
    "Steam нужно перезапустить",
    "Steam 需要重新启动",
];

pub const MAC_STEAM_NOT_CONFIGURED: Texts = [
    "Steam could not be configured: launch option copied",
    "Steam n'a pas pu être configuré : option de lancement copiée",
    "Steam konnte nicht eingerichtet werden: Startoption kopiert",
    "No se pudo configurar Steam: opción de lanzamiento copiada",
    "Impossibile configurare Steam: opzione di avvio copiata",
    "Steam を設定できませんでした。起動オプションをコピーしました",
    "Steam을 설정하지 못했습니다. 실행 옵션을 복사했습니다",
    "Nie udało się skonfigurować Steam: skopiowano opcję uruchamiania",
    "Não foi possível configurar a Steam: opção de inicialização copiada",
    "Не удалось настроить Steam: параметр запуска скопирован",
    "无法配置 Steam：已复制启动选项",
];

pub const MAC_STEAM_RESTART_TEXT: Texts = [
    "For Civilization VI to start with BetterGameExperience, the launcher adds its launch option in Steam. Steam only reads its settings when it starts: it will close, then reopen.",
    "Pour que Civilization VI se lance avec BetterGameExperience, le lanceur ajoute son option de lancement dans Steam. Steam ne relit ses réglages qu'au démarrage : il va se fermer, puis se rouvrir.",
    "Damit Civilization VI mit BetterGameExperience startet, fügt der Launcher seine Startoption in Steam hinzu. Steam liest seine Einstellungen nur beim Start: Es wird geschlossen und dann wieder geöffnet.",
    "Para que Civilization VI se inicie con BetterGameExperience, el lanzador añade su opción de lanzamiento en Steam. Steam solo lee su configuración al iniciarse: se cerrará y luego se volverá a abrir.",
    "Perché Civilization VI si avvii con BetterGameExperience, il launcher aggiunge la sua opzione di avvio in Steam. Steam legge le impostazioni solo all'avvio: si chiuderà e poi si riaprirà.",
    "Civilization VI を BetterGameExperience 付きで起動するため、ランチャーが Steam に起動オプションを追加します。Steam は起動時にしか設定を読み込まないため、いったん終了してから再び開きます。",
    "Civilization VI가 BetterGameExperience와 함께 실행되도록 런처가 Steam에 실행 옵션을 추가합니다. Steam은 시작할 때만 설정을 읽으므로 종료한 뒤 다시 열립니다.",
    "Aby Civilization VI uruchamiało się z BetterGameExperience, launcher dodaje swoją opcję uruchamiania w Steam. Steam wczytuje ustawienia tylko przy starcie: zamknie się, a potem otworzy ponownie.",
    "Para o Civilization VI iniciar com o BetterGameExperience, o iniciador adiciona a opção de inicialização dele na Steam. A Steam só lê as configurações ao iniciar: ela vai fechar e depois abrir de novo.",
    "Чтобы Civilization VI запускалась с BetterGameExperience, лаунчер добавляет свой параметр запуска в Steam. Steam читает настройки только при запуске: он закроется, а затем откроется снова.",
    "为了让 Civilization VI 随 BetterGameExperience 一起启动，启动器会在 Steam 中添加它的启动选项。Steam 只在启动时读取设置：它会先关闭，然后重新打开。",
];

pub const NOT_INSTALLED: Texts = [
    "not installed",
    "pas installé",
    "nicht installiert",
    "no instalado",
    "non installato",
    "未インストール",
    "설치되지 않음",
    "niezainstalowane",
    "não instalado",
    "не установлен",
    "未安装",
];

pub const NO_PUBLISHED_RELEASE: Texts = [
    "no published release",
    "aucune release publiée",
    "keine veröffentlichte Version",
    "ninguna versión publicada",
    "nessuna versione pubblicata",
    "公開されたリリースがありません",
    "공개된 릴리스가 없습니다",
    "brak opublikowanej wersji",
    "nenhuma versão publicada",
    "нет опубликованных выпусков",
    "没有已发布的版本",
];

pub const OLD_PROXY_IN_PLACE_OF_THE: Texts = [
    "old proxy in place of the original",
    "ancien proxy à la place de l'original",
    "alter Proxy anstelle des Originals",
    "proxy antiguo en lugar del original",
    "vecchio proxy al posto dell'originale",
    "元のファイルの代わりに古いプロキシ",
    "원본 대신 이전 프록시",
    "stare proxy zamiast oryginału",
    "proxy antigo no lugar do original",
    "старый прокси вместо оригинала",
    "旧代理替换了原始文件",
];

pub const PLEASE_WAIT: Texts = [
    "Please wait.",
    "Merci de patienter.",
    "Bitte warten.",
    "Espera, por favor.",
    "Attendere prego.",
    "お待ちください。",
    "잠시 기다려 주세요.",
    "Proszę czekać.",
    "Aguarde.",
    "Пожалуйста, подождите.",
    "请稍候。",
];

pub const PRE_RELEASE_INSTALL_TEST_VERSIONS: Texts = [
    "Pre-release: install test versions",
    "Pré-release : installer les versions de test",
    "Vorabversion: Testversionen installieren",
    "Versión preliminar: instalar las versiones de prueba",
    "Versione di prova: installa le versioni di test",
    "プレリリース: テスト版をインストールする",
    "사전 릴리스: 테스트 버전 설치",
    "Wersja testowa: instaluj wersje testowe",
    "Versão de teste: instalar as versões de teste",
    "Тестовые версии: устанавливать тестовые версии",
    "预发布版本：安装测试版本",
];

pub const PROXY_INSTALLED_OTHER_VERSION: Texts = [
    "proxy installed, other version",
    "proxy installé, autre version",
    "Proxy installiert, andere Version",
    "proxy instalado, otra versión",
    "proxy installato, altra versione",
    "プロキシはインストール済み (別のバージョン)",
    "프록시 설치됨, 다른 버전",
    "proxy zainstalowane, inna wersja",
    "proxy instalado, outra versão",
    "прокси установлен, другая версия",
    "代理已安装，版本不同",
];

pub const PROXY_UP_TO_DATE: Texts = [
    "proxy up to date",
    "proxy à jour",
    "Proxy aktuell",
    "proxy actualizado",
    "proxy aggiornato",
    "プロキシは最新",
    "프록시 최신",
    "proxy aktualne",
    "proxy atualizado",
    "прокси актуален",
    "代理为最新",
];

pub const REPAIR_REINSTALLS_VERSION_X: Texts = [
    "Repair\nReinstalls version {1}.",
    "Réparer\nRéinstalle la version {1}.",
    "Reparieren\nInstalliert Version {1} neu.",
    "Reparar\nVuelve a instalar la versión {1}.",
    "Ripara\nReinstalla la versione {1}.",
    "修復\nバージョン {1} を再インストールします。",
    "복구\n버전 {1}을(를) 다시 설치합니다.",
    "Napraw\nInstaluje ponownie wersję {1}.",
    "Reparar\nReinstala a versão {1}.",
    "Восстановить\nПереустанавливает версию {1}.",
    "修复\n重新安装版本 {1}。",
];

pub const RESTART_CIVILIZATION_VI_THROUGH_STEAM: Texts = [
    "Restart Civilization VI\nThrough Steam.",
    "Relancer Civilization VI\nPar Steam.",
    "Civilization VI neu starten\nÜber Steam.",
    "Reiniciar Civilization VI\nA través de Steam.",
    "Riavvia Civilization VI\nTramite Steam.",
    "Civilization VI を再起動\nSteam 経由で起動します。",
    "Civilization VI 다시 시작\nSteam을 통해 실행합니다.",
    "Uruchom ponownie Civilization VI\nPrzez Steam.",
    "Reiniciar o Civilization VI\nPela Steam.",
    "Перезапустить Civilization VI\nЧерез Steam.",
    "重新启动 Civilization VI\n通过 Steam 启动。",
];

pub const THE_GAME_HAS_ITS_ORIGINAL_FILES: Texts = [
    "The game has its original files back.",
    "Le jeu a retrouvé ses fichiers d'origine.",
    "Das Spiel hat wieder seine Originaldateien.",
    "El juego ha recuperado sus archivos originales.",
    "Il gioco ha di nuovo i suoi file originali.",
    "ゲームは元のファイルに戻りました。",
    "게임이 원래 파일로 돌아왔습니다.",
    "Gra odzyskała swoje oryginalne pliki.",
    "O jogo voltou a ter os arquivos originais.",
    "Игре возвращены её исходные файлы.",
    "游戏已恢复原始文件。",
];

pub const THE_OPERATION_FAILED: Texts = [
    "The operation failed",
    "L'opération a échoué",
    "Der Vorgang ist fehlgeschlagen",
    "La operación ha fallado",
    "L'operazione non è riuscita",
    "操作に失敗しました",
    "작업에 실패했습니다",
    "Operacja nie powiodła się",
    "A operação falhou",
    "Операция не удалась",
    "操作失败",
];

pub const UNINSTALLING_BETTERGAMEDLLS: Texts = [
    "Uninstalling BetterGameExperience…",
    "Désinstallation de BetterGameExperience…",
    "BetterGameExperience wird deinstalliert …",
    "Desinstalando BetterGameExperience…",
    "Disinstallazione di BetterGameExperience…",
    "BetterGameExperience をアンインストール中…",
    "BetterGameExperience를 제거하는 중…",
    "Odinstalowywanie BetterGameExperience…",
    "Desinstalando o BetterGameExperience…",
    "Удаление BetterGameExperience…",
    "正在卸载 BetterGameExperience…",
];

pub const UNINSTALL_RESTORES_THE_GAME_S_ORIGINAL: Texts = [
    "Uninstall\nRestores the game's original files.",
    "Désinstaller\nRemet les fichiers d'origine du jeu.",
    "Deinstallieren\nStellt die Originaldateien des Spiels wieder her.",
    "Desinstalar\nRestaura los archivos originales del juego.",
    "Disinstalla\nRipristina i file originali del gioco.",
    "アンインストール\nゲームの元のファイルに戻します。",
    "제거\n게임의 원래 파일로 되돌립니다.",
    "Odinstaluj\nPrzywraca oryginalne pliki gry.",
    "Desinstalar\nRestaura os arquivos originais do jogo.",
    "Удалить\nВозвращает исходные файлы игры.",
    "卸载\n恢复游戏的原始文件。",
];

pub const UNREADABLE_MANIFEST_SIGNATURE: Texts = [
    "unreadable manifest signature",
    "signature du manifeste illisible",
    "Signatur des Manifests unlesbar",
    "firma del manifiesto ilegible",
    "firma del manifesto illeggibile",
    "マニフェストの署名を読み込めません",
    "매니페스트 서명을 읽을 수 없습니다",
    "nieczytelny podpis manifestu",
    "assinatura do manifesto ilegível",
    "нечитаемая подпись манифеста",
    "无法读取清单签名",
];

pub const UNREADABLE_MANIFEST_X: Texts = [
    "unreadable manifest: {1}",
    "manifeste illisible : {1}",
    "Manifest unlesbar: {1}",
    "manifiesto ilegible: {1}",
    "manifesto illeggibile: {1}",
    "マニフェストを読み込めません: {1}",
    "매니페스트를 읽을 수 없습니다: {1}",
    "nieczytelny manifest: {1}",
    "manifesto ilegível: {1}",
    "нечитаемый манифест: {1}",
    "无法读取清单：{1}",
];

pub const UNREADABLE_RELEASE_LIST_X: Texts = [
    "unreadable release list: {1}",
    "liste des releases illisible : {1}",
    "Versionsliste unlesbar: {1}",
    "lista de versiones ilegible: {1}",
    "elenco delle versioni illeggibile: {1}",
    "リリース一覧を読み込めません: {1}",
    "릴리스 목록을 읽을 수 없습니다: {1}",
    "nieczytelna lista wersji: {1}",
    "lista de versões ilegível: {1}",
    "нечитаемый список выпусков: {1}",
    "无法读取版本列表：{1}",
];

pub const UPDATE_INSTALLS_VERSION_X: Texts = [
    "Update\nInstalls version {1}.",
    "Mettre à jour\nInstalle la version {1}.",
    "Aktualisieren\nInstalliert Version {1}.",
    "Actualizar\nInstala la versión {1}.",
    "Aggiorna\nInstalla la versione {1}.",
    "アップデート\nバージョン {1} をインストールします。",
    "업데이트\n버전 {1}을(를) 설치합니다.",
    "Aktualizuj\nInstaluje wersję {1}.",
    "Atualizar\nInstala a versão {1}.",
    "Обновить\nУстанавливает версию {1}.",
    "更新\n安装版本 {1}。",
];

pub const UPDATE_INSTALLS_VERSION_X_WHICH_NO: Texts = [
    "Update\nInstalls version {1}, which no longer replaces any game file: Steam no longer removes it.",
    "Mettre à jour\nInstalle la version {1}, qui ne remplace plus aucun fichier du jeu : Steam ne la retire plus.",
    "Aktualisieren\nInstalliert Version {1}, die keine Spieldatei mehr ersetzt: Steam entfernt sie nicht mehr.",
    "Actualizar\nInstala la versión {1}, que ya no sustituye ningún archivo del juego: Steam ya no la retira.",
    "Aggiorna\nInstalla la versione {1}, che non sostituisce più alcun file del gioco: Steam non la rimuove più.",
    "アップデート\nバージョン {1} をインストールします。このバージョンはゲームのファイルを置き換えないため、Steam に削除されなくなります。",
    "업데이트\n버전 {1}을(를) 설치합니다. 이 버전은 게임 파일을 바꾸지 않으므로 Steam이 더 이상 제거하지 않습니다.",
    "Aktualizuj\nInstaluje wersję {1}, która nie zastępuje już żadnego pliku gry: Steam już jej nie usuwa.",
    "Atualizar\nInstala a versão {1}, que não substitui mais nenhum arquivo do jogo: a Steam não a remove mais.",
    "Обновить\nУстанавливает версию {1}, которая больше не заменяет файлы игры: Steam её больше не удаляет.",
    "更新\n安装版本 {1}，它不再替换任何游戏文件：Steam 不会再移除它。",
];

pub const UPDATING_BETTERGAMEDLLS_TO_VERSION_X: Texts = [
    "Updating BetterGameExperience to version {1}…",
    "Mise à jour de BetterGameExperience vers la version {1}…",
    "BetterGameExperience wird auf Version {1} aktualisiert …",
    "Actualizando BetterGameExperience a la versión {1}…",
    "Aggiornamento di BetterGameExperience alla versione {1}…",
    "BetterGameExperience をバージョン {1} にアップデート中…",
    "BetterGameExperience를 버전 {1}(으)로 업데이트하는 중…",
    "Aktualizowanie BetterGameExperience do wersji {1}…",
    "Atualizando o BetterGameExperience para a versão {1}…",
    "Обновление BetterGameExperience до версии {1}…",
    "正在将 BetterGameExperience 更新到版本 {1}…",
];

pub const VERSION_INSTALLED_BY_THE_INSTALLER_X: Texts = [
    "Version installed by the installer: {1}",
    "Version posée par l'installeur : {1}",
    "Vom Installationsprogramm installierte Version: {1}",
    "Versión instalada por el instalador: {1}",
    "Versione installata dal programma di installazione: {1}",
    "インストーラーが導入したバージョン: {1}",
    "설치 프로그램이 설치한 버전: {1}",
    "Wersja zainstalowana przez instalator: {1}",
    "Versão instalada pelo instalador: {1}",
    "Версия, установленная установщиком: {1}",
    "安装程序安装的版本：{1}",
];

pub const VERSION_X_NEEDS_A_NEWER_INSTALLER: Texts = [
    "version {1} needs a newer installer: download it from the release page.",
    "la version {1} demande un installeur plus récent : télécharge-le depuis la page de la release.",
    "Version {1} benötigt ein neueres Installationsprogramm: Laden Sie es von der Seite der Version herunter.",
    "la versión {1} necesita un instalador más reciente: descárgalo desde la página de la versión.",
    "la versione {1} richiede un programma di installazione più recente: scaricalo dalla pagina della versione.",
    "バージョン {1} にはより新しいインストーラーが必要です。リリースのページからダウンロードしてください。",
    "버전 {1}에는 더 새로운 설치 프로그램이 필요합니다. 릴리스 페이지에서 내려받으세요.",
    "wersja {1} wymaga nowszego instalatora: pobierz go ze strony wersji.",
    "a versão {1} precisa de um instalador mais recente: baixe-o na página da versão.",
    "версии {1} нужен более новый установщик: загрузите его со страницы выпуска.",
    "版本 {1} 需要更新的安装程序：请从版本页面下载。",
];

pub const X_IS_MISSING_VERIFY_THE_GAME: Texts = [
    "{1} is missing: verify the game files in Steam.",
    "{1} manque : vérifie les fichiers du jeu dans Steam.",
    "{1} fehlt: Überprüfen Sie die Spieldateien in Steam.",
    "Falta {1}: verifica los archivos del juego en Steam.",
    "{1} manca: verifica i file del gioco in Steam.",
    "{1} がありません: Steam でゲームファイルの整合性を確認してください。",
    "{1}이(가) 없습니다: Steam에서 게임 파일 무결성을 확인하세요.",
    "Brak {1}: zweryfikuj pliki gry w Steam.",
    "{1} está faltando: verifique os arquivos do jogo na Steam.",
    "Отсутствует {1}: проверьте файлы игры в Steam.",
    "缺少 {1}：请在 Steam 中验证游戏文件。",
];

pub const X_IS_NOT_A_BETTERGAMEDLLS_FILE: Texts = [
    "{1} is not a BetterGameExperience file (another mod?): remove it, then run the installer again.",
    "{1} n'est pas un fichier de BetterGameExperience (un autre mod ?) : retire-le, puis relance l'installeur.",
    "{1} ist keine Datei von BetterGameExperience (ein anderer Mod?): Entfernen Sie sie und starten Sie dann das Installationsprogramm erneut.",
    "{1} no es un archivo de BetterGameExperience (¿otro mod?): elimínalo y vuelve a ejecutar el instalador.",
    "{1} non è un file di BetterGameExperience (un'altra mod?): rimuovilo, poi avvia di nuovo il programma di installazione.",
    "{1} は BetterGameExperience のファイルではありません (別の MOD?)。削除してから、インストーラーを再実行してください。",
    "{1}은(는) BetterGameExperience 파일이 아닙니다(다른 모드?). 삭제한 뒤 설치 프로그램을 다시 실행하세요.",
    "{1} nie jest plikiem BetterGameExperience (inny mod?): usuń go, a następnie uruchom ponownie instalator.",
    "{1} não é um arquivo do BetterGameExperience (outro mod?): remova-o e execute o instalador novamente.",
    "{1} не является файлом BetterGameExperience (другой мод?): удалите его, затем снова запустите установщик.",
    "{1} 不是 BetterGameExperience 的文件（其他模组？）：请删除它，然后重新运行安装程序。",
];

pub const X_IS_NOT_THE_CIVILIZATION_VI: Texts = [
    "{1} is not the Civilization VI (Gathering Storm) folder.",
    "{1} n'est pas le dossier de Civilization VI (Gathering Storm).",
    "{1} ist nicht der Ordner von Civilization VI (Gathering Storm).",
    "{1} no es la carpeta de Civilization VI (Gathering Storm).",
    "{1} non è la cartella di Civilization VI (Gathering Storm).",
    "{1} は Civilization VI (Gathering Storm) のフォルダーではありません。",
    "{1}은(는) Civilization VI (Gathering Storm) 폴더가 아닙니다.",
    "{1} nie jest folderem Civilization VI (Gathering Storm).",
    "{1} não é a pasta do Civilization VI (Gathering Storm).",
    "{1} не является папкой Civilization VI (Gathering Storm).",
    "{1} 不是 Civilization VI（Gathering Storm）的文件夹。",
];

pub const X_SHA_256_DIFFERS_FROM_THE: Texts = [
    "{1}: SHA-256 differs from the manifest",
    "{1} : SHA-256 différent de celui du manifeste",
    "{1}: SHA-256 weicht vom Manifest ab",
    "{1}: el SHA-256 no coincide con el del manifiesto",
    "{1}: SHA-256 diverso da quello del manifesto",
    "{1}: SHA-256 がマニフェストと一致しません",
    "{1}: SHA-256이 매니페스트와 다릅니다",
    "{1}: SHA-256 różni się od manifestu",
    "{1}: SHA-256 diferente do manifesto",
    "{1}: SHA-256 не совпадает с манифестом",
    "{1}：SHA-256 与清单不符",
];

pub const X_THE_GAME_WILL_START_WITHOUT: Texts = [
    "{1}\n\nThe game will start without the update.",
    "{1}\n\nLe jeu va se lancer sans mise à jour.",
    "{1}\n\nDas Spiel startet ohne das Update.",
    "{1}\n\nEl juego se iniciará sin la actualización.",
    "{1}\n\nIl gioco si avvierà senza l'aggiornamento.",
    "{1}\n\nアップデートせずにゲームを起動します。",
    "{1}\n\n업데이트 없이 게임을 시작합니다.",
    "{1}\n\nGra uruchomi się bez aktualizacji.",
    "{1}\n\nO jogo vai iniciar sem a atualização.",
    "{1}\n\nИгра запустится без обновления.",
    "{1}\n\n游戏将在不更新的情况下启动。",
];

pub const YOU_CAN_CLOSE_THIS_WINDOW: Texts = [
    "Everything is in place: you can close this window and launch Civilization VI. The game will tell you when an update is available.",
    "Tout est en place : tu peux fermer cette fenêtre et lancer Civilization VI. Le jeu te préviendra quand une mise à jour sera disponible.",
    "Alles ist bereit: Sie können dieses Fenster schließen und Civilization VI starten. Das Spiel meldet Ihnen, wenn ein Update verfügbar ist.",
    "Todo está listo: puedes cerrar esta ventana y lanzar Civilization VI. El juego te avisará cuando haya una actualización disponible.",
    "È tutto pronto: puoi chiudere questa finestra e avviare Civilization VI. Il gioco ti avviserà quando sarà disponibile un aggiornamento.",
    "準備が整いました。このウィンドウを閉じて Civilization VI を起動できます。アップデートがあるときはゲーム内でお知らせします。",
    "모든 준비가 끝났습니다. 이 창을 닫고 Civilization VI를 실행하세요. 업데이트가 있으면 게임에서 알려 드립니다.",
    "Wszystko gotowe: możesz zamknąć to okno i uruchomić Civilization VI. Gra powiadomi cię, gdy pojawi się aktualizacja.",
    "Tudo pronto: você pode fechar esta janela e iniciar o Civilization VI. O jogo avisará quando houver uma atualização disponível.",
    "Всё готово: можно закрыть это окно и запустить Civilization VI. Игра сообщит, когда выйдет обновление.",
    "一切就绪：你可以关闭此窗口并启动 Civilization VI。有可用更新时，游戏会通知你。",
];
