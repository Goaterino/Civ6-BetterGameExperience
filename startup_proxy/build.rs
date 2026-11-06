//! Relais des exports de `winmm.dll` : le proxy de démarrage prend le nom de cette DLL de Windows,
//! que l'exe importe et que le chargeur cherche d'abord dans le dossier de l'exe.
//!
//! Chaque export nommé de la vraie winmm (liste dans src/winmm_exports.txt, tirée de celle de
//! Windows 11) devient un relais `jmp [rip+case]`, et l'ordinal 2, sans nom, aussi. Les cases sont
//! remplies au chargement (`relais::init`) par les adresses de la winmm de System32, dont le chemin
//! est lu à ce moment-là : pas de chemin figé, contrairement aux renvois PE.
//!
//! La chaîne d'outils est GNU : les exports passent par un fichier .def donné à ld, en plus de celui
//! de rustc, qui associe chaque nom à son relais.

fn main() {
    let liste = std::fs::read_to_string("src/winmm_exports.txt").unwrap();
    let noms: Vec<&str> = liste.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // Entrées : (nom NUL-terminé, ordinal). L'ordinal 2 de winmm n'a pas de nom.
    let mut entrees: Vec<(Option<&str>, u16)> = noms.iter().map(|n| (Some(*n), 0)).collect();
    entrees.push((None, 2));

    let mut def = String::from("EXPORTS\n");
    let mut asm = String::new();
    let mut table = String::new();
    for (i, (nom, ordinal)) in entrees.iter().enumerate() {
        asm += &format!(".globl bgd_winmm_{i}\nbgd_winmm_{i}:\n    jmp qword ptr [rip + {{cases}} + {}]\n", 8 * i);
        match nom {
            Some(n) => {
                def += &format!("    {n} = bgd_winmm_{i}\n");
                table += &format!("    (Some(b\"{n}\0\"), 0),\n");
            }
            None => {
                def += &format!("    bgd_winmm_ord{ordinal} = bgd_winmm_{i} @{ordinal} NONAME\n");
                table += &format!("    (None, {ordinal}),\n");
            }
        }
    }
    let n = entrees.len();
    let rs = format!(
        "/// Exports relayés : nom NUL-terminé, ou ordinal sans nom.\n\
         pub const EXPORTS: [(Option<&[u8]>, u16); {n}] = [\n{table}];\n\n\
         /// Adresse de chaque export dans la vraie winmm, dans l'ordre de `EXPORTS`.\n\
         pub static mut CASES: [usize; {n}] = [0; {n}];\n\n\
         std::arch::global_asm!(\".text\n.p2align 4\n\", \"{}\", cases = sym CASES);\n",
        asm.replace('\n', "\n")
    );
    std::fs::write(out.join("winmm_relais.rs"), rs).unwrap();
    std::fs::write(out.join("winmm.def"), def).unwrap();
    println!("cargo:rustc-cdylib-link-arg={}", out.join("winmm.def").display());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/winmm_exports.txt");
}
