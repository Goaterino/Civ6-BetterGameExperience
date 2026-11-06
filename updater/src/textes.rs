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
