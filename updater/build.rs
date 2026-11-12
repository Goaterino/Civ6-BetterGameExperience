//! Ressources de l'exe (ressources/BetterGameExperience.rc) : manifeste (pas d'élévation au
//! lancement, comctl32 v6, versions de Windows, DPI) et informations de version. Un exe sans
//! manifeste ni description ressemble davantage, pour un antivirus, à un logiciel malveillant.
//!
//! Le compilateur de ressources est rc.exe pour la chaîne MSVC, celle de la release, qui l'exige ;
//! windres pour la chaîne GNU, qui peut ne pas l'avoir : l'exe s'en passe alors (gui.rs active
//! comctl32 v6 autrement).

fn main() {
    println!("cargo:rerun-if-changed=ressources");
    println!("cargo:rerun-if-env-changed=BGD_RELEASE");

    let release = std::env::var("BGD_RELEASE").unwrap_or_default();
    let numbers: Vec<u16> = release.split(['.', '-']).take(3).map_while(|n| n.parse().ok()).collect();
    let (major, minor, patch, text) = match numbers[..] {
        [a, b, c] => (a, b, c, release.as_str()),
        _ => (0, 0, 0, "0.0.0"),
    };
    let macros = [
        format!("VER_MAJOR={major}"),
        format!("VER_MINOR={minor}"),
        format!("VER_PATCH={patch}"),
        format!("VER_STR=\"{text}\""),
    ];
    let result = embed_resource::compile("ressources/BetterGameExperience.rc", &macros);

    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        // Le manifeste vient des ressources, pas de celui que link.exe écrirait à côté de l'exe.
        println!("cargo:rustc-link-arg-bins=/MANIFEST:NO");
        result.manifest_required().unwrap();
    } else {
        result.manifest_optional().unwrap();
    }
}
