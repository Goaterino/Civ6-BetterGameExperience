//! Images et sons des signaux d'équipe (`ping_assets/`), embarqués dans ce proxy et posés par lui
//! dans `DLC\BGEPings` au lancement du jeu, avant que le jeu ne cherche ses mods.
//!
//! L'installeur ne pose que les deux DLL : un installeur déjà distribué (0.10.0 et avant) refuse
//! tout manifeste qui en liste plus, et c'est lui que lance la mise à jour depuis le jeu. Les médias
//! suivent donc la DLL : une mise à jour de la DLL met à jour les médias au lancement suivant.
//!
//! Ils forment un mod local, « BGE - Ping assets », caché et décoché : l'hôte ne l'ajoute à la
//! configuration de la partie qu'au lancement, si tous les joueurs l'ont (fin de `salon.lua`, clé
//! `BGD_MEDIAS`). Un fichier identique n'est pas réécrit (la date du `.modinfo` ne change pas, le
//! jeu garde son cache des mods) ; le `.modinfo` est écrit en dernier, après les fichiers qu'il
//! cite. Un `BGEPings.modinfo` qui n'est pas le nôtre n'est pas touché. Sans droit d'écriture dans
//! le dossier du jeu, rien n'est posé : le salon le sait par `presents()`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::log;

/// Identifiant du mod, présent dans son `.modinfo`.
pub const MOD_ID: &str = "863cf39b-8e3e-4dd9-aab1-1b4d4a66dabd";
/// Dossier du mod, relatif à la racine du jeu.
pub const DOSSIER: &str = "DLC/BGEPings";

/// Chemin relatif au dossier du mod, et contenu. Le `.modinfo` en dernier.
pub const FICHIERS: [(&str, &[u8]); 13] = [
    ("Art/TPAssist.dds", include_bytes!("../../ping_assets/Art/TPAssist.dds")),
    ("Art/TPComing.dds", include_bytes!("../../ping_assets/Art/TPComing.dds")),
    ("Art/TPDanger.dds", include_bytes!("../../ping_assets/Art/TPDanger.dds")),
    ("Art/TPDenied.dds", include_bytes!("../../ping_assets/Art/TPDenied.dds")),
    ("Art/TPFocus.dds", include_bytes!("../../ping_assets/Art/TPFocus.dds")),
    ("Art/TPLook.dds", include_bytes!("../../ping_assets/Art/TPLook.dds")),
    ("Art/TPMissing.dds", include_bytes!("../../ping_assets/Art/TPMissing.dds")),
    ("Art/TPRipple.dds", include_bytes!("../../ping_assets/Art/TPRipple.dds")),
    ("Art/TPWheel.dds", include_bytes!("../../ping_assets/Art/TPWheel.dds")),
    ("Platforms/Windows/Audio/TeamPingsAudio.bnk", include_bytes!("../../ping_assets/Platforms/Windows/Audio/TeamPingsAudio.bnk")),
    ("Platforms/Windows/Audio/TeamPingsAudio.xml", include_bytes!("../../ping_assets/Platforms/Windows/Audio/TeamPingsAudio.xml")),
    ("Platforms/Windows/Audio/TeamPingsBanks.ini", include_bytes!("../../ping_assets/Platforms/Windows/Audio/TeamPingsBanks.ini")),
    ("BGEPings.modinfo", include_bytes!("../../ping_assets/BGEPings.modinfo")),
];

static PRESENTS: AtomicBool = AtomicBool::new(false);

/// Les médias de cette version sont-ils en place (posés ou déjà là) ?
pub fn presents() -> bool {
    PRESENTS.load(Ordering::Relaxed)
}

/// Racine du jeu : trois dossiers au-dessus de `Base\Binaries\Win64Steam`, si elle a bien
/// `DLC\Expansion2`.
pub fn racine(dossier_dll: &Path) -> Option<PathBuf> {
    let r = dossier_dll.parent()?.parent()?.parent()?.to_path_buf();
    r.join("DLC").join("Expansion2").is_dir().then_some(r)
}

/// Bilan d'une pose : fichiers écrits, fichiers déjà à jour.
#[derive(Debug, PartialEq)]
pub struct Bilan {
    pub ecrits: usize,
    pub a_jour: usize,
}

/// Pose les médias sous `racine` : chaque fichier absent ou différent est écrit à côté, puis
/// renommé par-dessus.
pub fn poser(racine: &Path) -> Result<Bilan, String> {
    let dossier = racine.join(DOSSIER);
    let modinfo = dossier.join("BGEPings.modinfo");
    if let Ok(texte) = std::fs::read(&modinfo) {
        if !texte.windows(MOD_ID.len()).any(|w| w == MOD_ID.as_bytes()) {
            return Err(format!("{} n'est pas le nôtre, laissé tel quel", modinfo.display()));
        }
    }
    let mut bilan = Bilan { ecrits: 0, a_jour: 0 };
    for (relatif, contenu) in FICHIERS {
        let chemin = dossier.join(relatif);
        if std::fs::read(&chemin).is_ok_and(|actuel| actuel == contenu) {
            bilan.a_jour += 1;
            continue;
        }
        let parent = chemin.parent().ok_or("chemin sans dossier")?;
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        let temporaire = chemin.with_extension("bge-new");
        std::fs::write(&temporaire, contenu).map_err(|e| format!("{}: {e}", temporaire.display()))?;
        if let Err(e) = std::fs::rename(&temporaire, &chemin) {
            let _ = std::fs::remove_file(&temporaire);
            return Err(format!("{}: {e}", chemin.display()));
        }
        bilan.ecrits += 1;
    }
    Ok(bilan)
}

/// Au chargement du proxy dans le jeu : pose les médias à côté du jeu, et le note.
pub fn installer() {
    let Some(racine) = crate::self_dir().as_deref().and_then(racine) else {
        log("médias : dossier du jeu introuvable, signaux sans images ni sons");
        return;
    };
    match poser(&racine) {
        Ok(b) => {
            PRESENTS.store(true, Ordering::Relaxed);
            if b.ecrits > 0 {
                log(&format!("médias : images et sons des signaux posés dans {DOSSIER} ({} fichiers écrits, {} déjà à jour)", b.ecrits, b.a_jour));
            } else {
                log(&format!("médias : images et sons des signaux à jour ({} fichiers)", b.a_jour));
            }
        }
        Err(e) => log(&format!("médias : images et sons des signaux non posés : {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jeu(nom: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bge_medias_{nom}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("DLC/Expansion2")).unwrap();
        std::fs::create_dir_all(dir.join("Base/Binaries/Win64Steam")).unwrap();
        dir
    }

    #[test]
    fn fichiers_du_depot() {
        let racine = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ping_assets");
        let mut attendus: Vec<String> = Vec::new();
        let mut pile = vec![racine.clone()];
        while let Some(d) = pile.pop() {
            for e in std::fs::read_dir(d).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    pile.push(p);
                } else {
                    attendus.push(p.strip_prefix(&racine).unwrap().to_string_lossy().replace('\\', "/"));
                }
            }
        }
        attendus.sort();
        let mut embarques: Vec<String> = FICHIERS.iter().map(|(r, _)| r.to_string()).collect();
        embarques.sort();
        assert_eq!(attendus, embarques, "ping_assets/ et FICHIERS doivent lister les mêmes fichiers");
        assert_eq!(FICHIERS.last().unwrap().0, "BGEPings.modinfo");
        let modinfo = std::str::from_utf8(FICHIERS.last().unwrap().1).unwrap();
        assert!(modinfo.contains(MOD_ID) && modinfo.contains("<EnabledByDefault>0") && modinfo.contains("AlwaysHidden"));
    }

    #[test]
    fn racine_depuis_le_dossier_de_la_dll() {
        let dir = jeu("racine");
        assert_eq!(racine(&dir.join("Base/Binaries/Win64Steam")), Some(dir.clone()));
        std::fs::remove_dir_all(dir.join("DLC")).unwrap();
        assert_eq!(racine(&dir.join("Base/Binaries/Win64Steam")), None);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn pose_mise_a_jour_et_mod_etranger() {
        let dir = jeu("pose");
        assert_eq!(poser(&dir).unwrap(), Bilan { ecrits: 13, a_jour: 0 });
        for (relatif, contenu) in FICHIERS {
            assert_eq!(std::fs::read(dir.join(DOSSIER).join(relatif)).unwrap(), contenu);
        }
        // Rien à réécrire : la date du .modinfo ne bouge pas.
        assert_eq!(poser(&dir).unwrap(), Bilan { ecrits: 0, a_jour: 13 });
        // Un fichier abîmé ou d'une autre version est remplacé, les autres laissés.
        std::fs::write(dir.join(DOSSIER).join("Art/TPWheel.dds"), b"ancien").unwrap();
        std::fs::remove_file(dir.join(DOSSIER).join("Platforms/Windows/Audio/TeamPingsBanks.ini")).unwrap();
        assert_eq!(poser(&dir).unwrap(), Bilan { ecrits: 2, a_jour: 11 });
        let restes: Vec<_> = std::fs::read_dir(dir.join(DOSSIER).join("Art")).unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".bge-new")).collect();
        assert!(restes.is_empty());
        // Un autre mod sous ce nom : rien n'est touché.
        std::fs::write(dir.join(DOSSIER).join("BGEPings.modinfo"), b"<Mod id=\"autre\"/>").unwrap();
        std::fs::write(dir.join(DOSSIER).join("Art/TPWheel.dds"), b"autre").unwrap();
        assert!(poser(&dir).is_err());
        assert_eq!(std::fs::read(dir.join(DOSSIER).join("Art/TPWheel.dds")).unwrap(), b"autre");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
