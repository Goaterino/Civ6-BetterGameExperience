//! Fenêtres de l'installeur : le TaskDialog de Windows. Il vient de comctl32 v6, que l'exe n'active
//! pas par un manifeste embarqué (la chaîne GNU n'a pas windres) : `init` l'active par un contexte
//! d'activation, à partir d'un manifeste écrit dans le dossier de travail.
#![allow(non_snake_case, clippy::upper_case_acronyms)]

use std::cell::Cell;
use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use crate::lang::{self, t, LANGUAGES};
use crate::win::{self, wide, SendMessageW};

#[repr(C, packed)]
struct TASKDIALOG_BUTTON {
    nButtonID: i32,
    pszButtonText: *const u16,
}

// commctrl.h déclare ces structures avec un alignement de 1.
#[repr(C, packed)]
struct TASKDIALOGCONFIG {
    cbSize: u32,
    hwndParent: *mut c_void,
    hInstance: *mut c_void,
    dwFlags: i32,
    dwCommonButtons: i32,
    pszWindowTitle: *const u16,
    pszMainIcon: *const u16,
    pszMainInstruction: *const u16,
    pszContent: *const u16,
    cButtons: u32,
    pButtons: *const TASKDIALOG_BUTTON,
    nDefaultButton: i32,
    cRadioButtons: u32,
    pRadioButtons: *const TASKDIALOG_BUTTON,
    nDefaultRadioButton: i32,
    pszVerificationText: *const u16,
    pszExpandedInformation: *const u16,
    pszExpandedControlText: *const u16,
    pszCollapsedControlText: *const u16,
    pszFooterIcon: *const u16,
    pszFooter: *const u16,
    pfCallback: Option<Callback>,
    lpCallbackData: isize,
    cxWidth: u32,
}

type Callback = unsafe extern "system" fn(*mut c_void, u32, usize, isize, isize) -> i32;
type TaskDialogIndirect = unsafe extern "system" fn(*const TASKDIALOGCONFIG, *mut i32, *mut i32, *mut i32) -> i32;

const TDF_USE_COMMAND_LINKS: i32 = 0x10;
const TDF_SHOW_MARQUEE_PROGRESS_BAR: i32 = 0x400;
const TDF_CALLBACK_TIMER: i32 = 0x800;
const TDF_VERIFICATION_FLAG_CHECKED: i32 = 0x100;
const TDCBF_OK_BUTTON: i32 = 0x1;
const TDCBF_CLOSE_BUTTON: i32 = 0x20;
const TDN_CREATED: u32 = 0;
const TDN_BUTTON_CLICKED: u32 = 2;
const TDN_TIMER: u32 = 4;
const TDN_DESTROYED: u32 = 5;
const TDN_RADIO_BUTTON_CLICKED: u32 = 6;
const TDN_VERIFICATION_CLICKED: u32 = 8;
const IDCLOSE: usize = 8;
const WM_USER: u32 = 0x400;
const TDM_CLICK_BUTTON: u32 = WM_USER + 102;
const TDM_SET_PROGRESS_BAR_MARQUEE: u32 = WM_USER + 107;
const TDM_ENABLE_BUTTON: u32 = WM_USER + 111;
const S_OK: i32 = 0;
const S_FALSE: i32 = 1;

/// Bouton de la fenêtre de progression, grisé jusqu'à la fin du travail.
const WAIT_BUTTON: i32 = 1000;
/// Boutons radio des langues : `RADIO_BASE` + rang dans `LANGUAGES`.
const RADIO_BASE: i32 = 2000;

/// Rendu par `ask` quand le joueur a changé de langue : la fenêtre est à reconstruire.
pub const LANGUAGE_CHANGED: i32 = -1;
/// Rendu par `ask` quand le joueur a coché ou décoché la case de la page.
pub const TOGGLED: i32 = -2;

const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency><dependentAssembly>
    <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
      processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
  </dependentAssembly></dependency>
</assembly>
"#;

static TASK_DIALOG: OnceLock<Option<TaskDialogIndirect>> = OnceLock::new();

#[derive(Clone, Copy)]
pub enum Icon {
    Info,
    Warning,
    Error,
}

pub struct Page<'a> {
    pub icon: Icon,
    pub instruction: &'a str,
    pub content: &'a str,
    /// Repliées sous « Détails ».
    pub details: Option<&'a str>,
    /// Liens de commande : identifiant, puis titre et description séparés par `\n`.
    pub buttons: &'a [(i32, &'a str)],
    /// Case à cocher sous les boutons, et son état : la cocher ou la décocher ferme la fenêtre.
    pub checkbox: Option<(&'a str, bool)>,
}

struct Ctx<'a> {
    /// Vrai pour un bouton qui agit sans fermer la fenêtre.
    keep_open: &'a dyn Fn(i32) -> bool,
    /// Fenêtre de progression : fin du travail.
    done: Option<&'a AtomicBool>,
    /// Le joueur a choisi une autre langue.
    relang: Cell<bool>,
    /// Le joueur a coché ou décoché la case.
    toggled: Cell<bool>,
}

fn load() -> Option<TaskDialogIndirect> {
    unsafe { win::SetProcessDPIAware() };
    let dir = crate::release::work_dir();
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("comctl32.manifest");
    std::fs::write(&path, MANIFEST).ok()?;
    let source = wide(&path.to_string_lossy());
    let mut ctx: win::ACTCTXW = unsafe { std::mem::zeroed() };
    ctx.cbSize = std::mem::size_of::<win::ACTCTXW>() as u32;
    ctx.lpSource = source.as_ptr();
    unsafe {
        let handle = win::CreateActCtxW(&ctx);
        let mut cookie = 0;
        if handle as isize == -1 || win::ActivateActCtx(handle, &mut cookie) == 0 {
            return None;
        }
        let comctl = win::LoadLibraryW(wide("comctl32.dll").as_ptr());
        let f = win::GetProcAddress(comctl, c"TaskDialogIndirect".as_ptr().cast());
        (!f.is_null()).then(|| std::mem::transmute::<*mut c_void, TaskDialogIndirect>(f))
    }
}

unsafe extern "system" fn callback(hwnd: *mut c_void, msg: u32, wparam: usize, _: isize, data: isize) -> i32 {
    let ctx = &*(data as *const Ctx);
    let finished = ctx.done.map(|d| d.load(Ordering::Acquire));
    match msg {
        TDN_CREATED => {
            win::OWNER.store(hwnd, Ordering::Relaxed);
            if finished.is_some() {
                SendMessageW(hwnd, TDM_SET_PROGRESS_BAR_MARQUEE, 1, 0);
                SendMessageW(hwnd, TDM_ENABLE_BUTTON, WAIT_BUTTON as usize, 0);
            }
        }
        TDN_TIMER if finished == Some(true) => {
            SendMessageW(hwnd, TDM_ENABLE_BUTTON, WAIT_BUTTON as usize, 1);
            SendMessageW(hwnd, TDM_CLICK_BUTTON, WAIT_BUTTON as usize, 0);
        }
        TDN_BUTTON_CLICKED => match finished {
            Some(false) => return S_FALSE,
            None if (ctx.keep_open)(wparam as i32) => return S_FALSE,
            _ => {}
        },
        TDN_RADIO_BUTTON_CLICKED => {
            let index = (wparam as i32 - RADIO_BASE) as usize;
            if index < LANGUAGES.len() && index != lang::current() {
                lang::set(index);
                ctx.relang.set(true);
                SendMessageW(hwnd, TDM_CLICK_BUTTON, IDCLOSE, 0);
            }
        }
        TDN_VERIFICATION_CLICKED => {
            ctx.toggled.set(true);
            SendMessageW(hwnd, TDM_CLICK_BUTTON, IDCLOSE, 0);
        }
        TDN_DESTROYED => win::OWNER.store(null_mut(), Ordering::Relaxed),
        _ => {}
    }
    S_OK
}

fn run(page: &Page, flags: i32, common: i32, radios: bool, ctx: &Ctx) -> i32 {
    let Some(task_dialog) = *TASK_DIALOG.get_or_init(load) else {
        return 0;
    };
    let icon = match page.icon {
        Icon::Warning => 0xFFFF,
        Icon::Error => 0xFFFE,
        Icon::Info => 0xFFFD,
    } as *const u16;
    let title = wide("BetterGameDLLs");
    let instruction = wide(page.instruction);
    let content = wide(page.content);
    let details = page.details.map(wide);
    let (expand, collapse) = (wide(t(["Masquer les détails", "Hide details"])), wide(t(["Détails", "Details"])));
    let texts: Vec<Vec<u16>> = page.buttons.iter().map(|(_, text)| wide(text)).collect();
    let buttons: Vec<TASKDIALOG_BUTTON> = page
        .buttons
        .iter()
        .zip(&texts)
        .map(|((id, _), text)| TASKDIALOG_BUTTON { nButtonID: *id, pszButtonText: text.as_ptr() })
        .collect();
    let mut cfg: TASKDIALOGCONFIG = unsafe { std::mem::zeroed() };
    cfg.cbSize = std::mem::size_of::<TASKDIALOGCONFIG>() as u32;
    let checkbox = page.checkbox.map(|(text, _)| wide(text));
    cfg.dwFlags = flags | if page.checkbox.is_some_and(|(_, on)| on) { TDF_VERIFICATION_FLAG_CHECKED } else { 0 };
    if let Some(text) = &checkbox {
        cfg.pszVerificationText = text.as_ptr();
    }
    cfg.dwCommonButtons = common;
    cfg.pszWindowTitle = title.as_ptr();
    cfg.pszMainIcon = icon;
    cfg.pszMainInstruction = instruction.as_ptr();
    cfg.pszContent = content.as_ptr();
    cfg.cButtons = buttons.len() as u32;
    cfg.pButtons = buttons.as_ptr();
    let labels: Vec<Vec<u16>> = LANGUAGES.iter().map(|l| wide(l.label)).collect();
    let radio_buttons: Vec<TASKDIALOG_BUTTON> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| TASKDIALOG_BUTTON { nButtonID: RADIO_BASE + i as i32, pszButtonText: label.as_ptr() })
        .collect();
    if radios {
        cfg.cRadioButtons = radio_buttons.len() as u32;
        cfg.pRadioButtons = radio_buttons.as_ptr();
        cfg.nDefaultRadioButton = RADIO_BASE + lang::current() as i32;
    }
    if let Some(details) = &details {
        cfg.pszExpandedInformation = details.as_ptr();
        cfg.pszExpandedControlText = expand.as_ptr();
        cfg.pszCollapsedControlText = collapse.as_ptr();
    }
    cfg.pfCallback = Some(callback);
    cfg.lpCallbackData = ctx as *const Ctx as isize;
    // En unités de dialogue : assez large pour les chemins du jeu, que la fenêtre abrège sinon.
    cfg.cxWidth = 320;
    let mut pressed = 0;
    unsafe { task_dialog(&cfg, &mut pressed, null_mut(), null_mut()) };
    if ctx.relang.get() {
        LANGUAGE_CHANGED
    } else if ctx.toggled.get() {
        TOGGLED
    } else {
        pressed
    }
}

/// Fenêtre à liens de commande, choix de la langue et bouton Fermer ; renvoie l'identifiant du
/// bouton choisi, `LANGUAGE_CHANGED` ou `TOGGLED`.
pub fn ask(page: &Page, keep_open: &dyn Fn(i32) -> bool) -> i32 {
    let flags = if page.buttons.is_empty() { 0 } else { TDF_USE_COMMAND_LINKS };
    run(page, flags, TDCBF_CLOSE_BUTTON, true, &Ctx { keep_open, done: None, relang: Cell::new(false), toggled: Cell::new(false) })
}

pub fn message(icon: Icon, instruction: &str, content: &str) {
    let page = Page { icon, instruction, content, details: None, buttons: &[], checkbox: None };
    run(&page, 0, TDCBF_OK_BUTTON, false, &Ctx { keep_open: &|_| false, done: None, relang: Cell::new(false), toggled: Cell::new(false) });
}

/// Fait `work` sur un autre thread, derrière une fenêtre de progression qui se ferme à la fin.
pub fn progress(instruction: &str, work: impl FnOnce() -> Result<(), String> + Send) -> Result<(), String> {
    let done = AtomicBool::new(false);
    std::thread::scope(|s| {
        let worker = s.spawn(|| {
            let result = work();
            done.store(true, Ordering::Release);
            result
        });
        let page = Page {
            icon: Icon::Info,
            instruction,
            content: t(["Merci de patienter.", "Please wait."]),
            details: None,
            buttons: &[(WAIT_BUTTON, t(["Fermer", "Close"]))],
            checkbox: None,
        };
        let flags = TDF_SHOW_MARQUEE_PROGRESS_BAR | TDF_CALLBACK_TIMER;
        run(&page, flags, 0, false, &Ctx { keep_open: &|_| false, done: Some(&done), relang: Cell::new(false), toggled: Cell::new(false) });
        worker.join().unwrap()
    })
}
