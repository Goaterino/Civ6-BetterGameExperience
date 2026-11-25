//! Fenêtres du lanceur sous macOS : `NSAlert` d'AppKit, avec la même interface que windows/gui.rs (le
//! TaskDialog de Windows), pour que macos/app.rs suive windows/app.rs.
//!
//! Correspondances avec le TaskDialog :
//! - liens de commande (`titre\ndescription`) : un bouton par titre, les descriptions sous le texte ;
//! - « Détails » repliés : texte secondaire, en petit, sous la fenêtre (deux ou trois lignes) ;
//! - case à cocher, et liens de langue en bas : une case et un menu dans la vue accessoire. Les
//!   changer ferme aussitôt la fenêtre (`stopModal` de NSApp, pris pour cible), comme sous Windows ;
//! - fenêtre de progression : session modale, barre indéterminée, bouton Fermer grisé jusqu'à la fin.
//!
//! Tout se fait sur le thread principal, que demande AppKit : seul le travail de `progress` va sur
//! un autre thread.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use objc2::rc::Retained;
use objc2::sel;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSApplication, NSApplicationActivationPolicy, NSButton, NSColor,
    NSControlStateValueOff, NSControlStateValueOn, NSFont, NSModalResponseStop, NSPopUpButton, NSProgressIndicator,
    NSProgressIndicatorStyle, NSTextField, NSView,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use crate::lang::{self, t, LANGUAGES};
use crate::textes as tx;

/// Rendu par `ask` quand le joueur a changé de langue : la fenêtre est à reconstruire.
pub const LANGUAGE_CHANGED: i32 = -1;
/// Rendu par `ask` quand le joueur a coché ou décoché la case de la page.
pub const TOGGLED: i32 = -2;

/// Largeur de la vue accessoire, en points : assez pour les chemins du jeu.
const WIDTH: f64 = 420.0;
const SPACING: f64 = 8.0;

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
    /// Repliées sous « Détails » sous Windows ; ici, en petit sous la fenêtre.
    pub details: Option<&'a str>,
    /// Liens de commande : identifiant, puis titre et description séparés par `\n`.
    pub buttons: &'a [(i32, &'a str)],
    /// Case à cocher sous les boutons, et son état : la cocher ou la décocher ferme la fenêtre.
    pub checkbox: Option<(&'a str, bool)>,
}

fn mtm() -> MainThreadMarker {
    MainThreadMarker::new().expect("fenêtres AppKit : thread principal seulement")
}

/// L'application, au premier plan : lancé par Steam ou par la notification du jeu, le lanceur doit
/// passer devant.
fn app(mtm: MainThreadMarker) -> Retained<NSApplication> {
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
    app
}

fn alert(mtm: MainThreadMarker, icon: Icon, instruction: &str, content: &str) -> Retained<NSAlert> {
    let alert = NSAlert::new(mtm);
    alert.setAlertStyle(match icon {
        Icon::Info => NSAlertStyle::Informational,
        Icon::Warning => NSAlertStyle::Warning,
        Icon::Error => NSAlertStyle::Critical,
    });
    alert.setMessageText(&NSString::from_str(instruction));
    alert.setInformativeText(&NSString::from_str(content));
    alert
}

/// Titre d'un lien de commande (première ligne), et sa description (le reste).
fn split_button(text: &str) -> (&str, Option<&str>) {
    match text.split_once('\n') {
        Some((title, desc)) if !desc.trim().is_empty() => (title, Some(desc.trim())),
        Some((title, _)) => (title, None),
        None => (text, None),
    }
}

/// Texte de la fenêtre : le contenu, puis la description de chaque bouton.
fn informative(content: &str, buttons: &[(i32, &str)]) -> String {
    let mut out = content.trim_end().to_string();
    for (_, text) in buttons {
        if let (title, Some(desc)) = split_button(text) {
            out += &format!("\n\n{title} : {desc}");
        }
    }
    out
}

/// Empile `views` de haut en bas dans une vue de largeur `WIDTH`.
fn stack(mtm: MainThreadMarker, views: &[Retained<NSView>]) -> Retained<NSView> {
    let height: f64 = views.iter().map(|v| v.frame().size.height).sum::<f64>() + SPACING * views.len().saturating_sub(1) as f64;
    let container = NSView::initWithFrame(NSView::alloc(mtm), NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(WIDTH, height)));
    let mut top = height;
    for v in views {
        let h = v.frame().size.height;
        top -= h;
        v.setFrameOrigin(NSPoint::new(0.0, top));
        container.addSubview(v);
        top -= SPACING;
    }
    container
}

fn details_label(mtm: MainThreadMarker, text: &str) -> Retained<NSView> {
    let label = NSTextField::wrappingLabelWithString(&NSString::from_str(text), mtm);
    label.setFont(Some(&NSFont::systemFontOfSize(NSFont::smallSystemFontSize())));
    label.setTextColor(Some(&NSColor::secondaryLabelColor()));
    label.setSelectable(true);
    label.setPreferredMaxLayoutWidth(WIDTH);
    let size = label.fittingSize();
    label.setFrameSize(NSSize::new(WIDTH, size.height));
    Retained::into_super(Retained::into_super(label))
}

/// Fenêtre à boutons, choix de la langue et bouton Fermer ; renvoie l'identifiant du bouton choisi,
/// `LANGUAGE_CHANGED`, `TOGGLED`, ou 0 pour Fermer.
pub fn ask(page: &Page, _keep_open: &dyn Fn(i32) -> bool) -> i32 {
    let mtm = mtm();
    let app = app(mtm);
    let alert = alert(mtm, page.icon, page.instruction, &informative(page.content, page.buttons));
    for (_, text) in page.buttons {
        alert.addButtonWithTitle(&NSString::from_str(split_button(text).0));
    }
    alert.addButtonWithTitle(&NSString::from_str(t(tx::CLOSE)));

    let mut views: Vec<Retained<NSView>> = Vec::new();
    if let Some(details) = page.details.filter(|d| !d.is_empty()) {
        views.push(details_label(mtm, details));
    }
    // La case et le menu ferment la fenêtre dès qu'on les change : leur action est `stopModal` de
    // NSApp, et `runModal` rend alors `NSModalResponseStop`.
    let checkbox = page.checkbox.map(|(text, on)| {
        let b = unsafe { NSButton::checkboxWithTitle_target_action(&NSString::from_str(text), Some(&app), Some(sel!(stopModal)), mtm) };
        b.setState(if on { NSControlStateValueOn } else { NSControlStateValueOff });
        b.sizeToFit();
        b
    });
    if let Some(b) = &checkbox {
        views.push(Retained::into_super(Retained::into_super(b.clone())));
    }
    let popup = NSPopUpButton::initWithFrame_pullsDown(
        NSPopUpButton::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(200.0, 26.0)),
        false,
    );
    for l in &LANGUAGES {
        popup.addItemWithTitle(&NSString::from_str(l.label));
    }
    popup.selectItemAtIndex(lang::current() as isize);
    unsafe {
        popup.setTarget(Some(&app));
        popup.setAction(Some(sel!(stopModal)));
    }
    views.push(Retained::into_super(Retained::into_super(Retained::into_super(popup.clone()))));
    alert.setAccessoryView(Some(&stack(mtm, &views)));

    loop {
        let response = alert.runModal();
        // Un `stopModal` laisse la fenêtre ouverte : on la ferme dans tous les cas.
        alert.window().orderOut(None);
        if response == NSModalResponseStop {
            let chosen = popup.indexOfSelectedItem();
            if chosen >= 0 && chosen as usize != lang::current() {
                lang::set(chosen as usize);
                return LANGUAGE_CHANGED;
            }
            if let (Some(b), Some((_, on))) = (&checkbox, page.checkbox) {
                if (b.state() == NSControlStateValueOn) != on {
                    return TOGGLED;
                }
            }
            continue;
        }
        let index = (response - NSAlertFirstButtonReturn) as usize;
        return page.buttons.get(index).map_or(0, |(id, _)| *id);
    }
}

pub fn message(icon: Icon, instruction: &str, content: &str) {
    let mtm = mtm();
    let _app = app(mtm);
    let alert = alert(mtm, icon, instruction, content);
    alert.runModal();
}

/// Fait `work` sur un autre thread, derrière une fenêtre de progression qui se ferme à la fin.
pub fn progress(instruction: &str, work: impl FnOnce() -> Result<(), String> + Send) -> Result<(), String> {
    let mtm = mtm();
    let app = app(mtm);
    let done = AtomicBool::new(false);
    std::thread::scope(|s| {
        let worker = s.spawn(|| {
            let result = work();
            done.store(true, Ordering::Release);
            result
        });
        let alert = alert(mtm, Icon::Info, instruction, t(tx::PLEASE_WAIT));
        let close = alert.addButtonWithTitle(&NSString::from_str(t(tx::CLOSE)));
        close.setEnabled(false);
        let bar = NSProgressIndicator::initWithFrame(
            NSProgressIndicator::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(WIDTH, 20.0)),
        );
        bar.setStyle(NSProgressIndicatorStyle::Bar);
        bar.setIndeterminate(true);
        unsafe { bar.startAnimation(None) };
        alert.setAccessoryView(Some(&bar));
        alert.layout();
        let window = alert.window();
        let session = app.beginModalSessionForWindow(&window);
        while !done.load(Ordering::Acquire) {
            unsafe { app.runModalSession(session) };
            std::thread::sleep(Duration::from_millis(30));
        }
        unsafe { app.endModalSession(session) };
        window.orderOut(None);
        worker.join().unwrap()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liens_de_commande() {
        assert_eq!(split_button("Installer\nTélécharge la 1.0"), ("Installer", Some("Télécharge la 1.0")));
        assert_eq!(split_button("Fermer"), ("Fermer", None));
        let text = informative("Jeu : x", &[(1, "Installer\nTélécharge"), (2, "Fermer")]);
        assert_eq!(text, "Jeu : x\n\nInstaller : Télécharge");
    }
}
