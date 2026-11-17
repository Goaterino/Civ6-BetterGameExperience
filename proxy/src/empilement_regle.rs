//! Unités empilées : la règle de l'excédent, commune à Windows (empilement.rs) et macOS
//! (empilement_mac.rs). Voir l'en-tête d'empilement.rs.

/// `~CRC32` des noms, comme `DB.MakeHash` (vérifié dans `1807d8a70`).
pub const FORMATION_CLASS_LAND_COMBAT: u32 = 0x9ad4_85f8;
pub const FORMATION_CLASS_NAVAL: u32 = 0xfeeb_5b2f;

/// Ce que la règle regarde d'une unité.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Desc {
    pub proprietaire: i32,
    pub classe: u32,
    pub x: i32,
    pub y: i32,
    pub vivante: bool,
    pub force: bool,
}

/// Seules les classes limitées à une unité par case et qui comptent dans les bonus.
pub fn classe_limitee(classe: u32) -> bool {
    classe == FORMATION_CLASS_LAND_COMBAT || classe == FORMATION_CLASS_NAVAL
}

pub fn meme_groupe(u: &Desc, v: &Desc) -> bool {
    classe_limitee(u.classe) && v.proprietaire == u.proprietaire && v.classe == u.classe
}

/// `u` est-elle en excédent ? `precedents` : les unités placées avant elle dans la liste de sa case.
pub fn excedent(u: &Desc, porteur: Option<&Desc>, precedents: impl IntoIterator<Item = Desc>) -> bool {
    if !classe_limitee(u.classe) {
        return false;
    }
    if let Some(p) = porteur {
        if meme_groupe(u, p) && (p.x, p.y) == (u.x, u.y) {
            return true;
        }
    }
    precedents.into_iter().any(|v| meme_groupe(u, &v) && v.vivante && v.force)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOUTIEN: u32 = 0xf1c3_2241;
    const AIR: u32 = 0x31b7_00cb;

    fn d(proprietaire: i32, classe: u32, x: i32, y: i32) -> Desc {
        Desc { proprietaire, classe, x, y, vivante: true, force: true }
    }

    #[test]
    fn hachages_des_classes() {
        let h = |s: &str| {
            // ~CRC32 (polynôme réfléchi 0xedb88320), comme DB.MakeHash.
            let mut c = 0xffff_ffffu32;
            for b in s.bytes() {
                c ^= b as u32;
                for _ in 0..8 {
                    c = if c & 1 != 0 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 };
                }
            }
            c
        };
        assert_eq!(h("FORMATION_CLASS_LAND_COMBAT"), FORMATION_CLASS_LAND_COMBAT);
        assert_eq!(h("FORMATION_CLASS_NAVAL"), FORMATION_CLASS_NAVAL);
        assert_eq!(h("FORMATION_CLASS_SUPPORT"), SOUTIEN);
        assert_eq!(h("FORMATION_CLASS_AIR"), AIR);
    }

    #[test]
    fn premiere_unite_comptee_seconde_en_excedent() {
        let a = d(0, FORMATION_CLASS_LAND_COMBAT, 5, 5);
        assert!(!excedent(&a, None, []));
        assert!(excedent(&a, None, [a]));
    }

    #[test]
    fn precedent_mort_ou_sans_force_ne_compte_pas() {
        let a = d(0, FORMATION_CLASS_LAND_COMBAT, 5, 5);
        assert!(!excedent(&a, None, [Desc { vivante: false, ..a }]));
        assert!(!excedent(&a, None, [Desc { force: false, ..a }]));
    }

    #[test]
    fn autres_proprietaires_et_classes_ne_comptent_pas() {
        let a = d(0, FORMATION_CLASS_LAND_COMBAT, 5, 5);
        assert!(!excedent(&a, None, [d(1, FORMATION_CLASS_LAND_COMBAT, 5, 5)]));
        assert!(!excedent(&a, None, [d(0, FORMATION_CLASS_NAVAL, 5, 5), d(0, SOUTIEN, 5, 5)]));
        let n = d(0, FORMATION_CLASS_NAVAL, 5, 5);
        assert!(excedent(&n, None, [a, n]));
    }

    #[test]
    fn classes_non_limitees_jamais_en_excedent() {
        for c in [SOUTIEN, AIR, 0xc251_0aa7] {
            let u = d(0, c, 5, 5);
            assert!(!excedent(&u, Some(&u), [u, u]));
        }
    }

    #[test]
    fn porteur_sur_la_case_occupe_la_place() {
        let a = d(0, FORMATION_CLASS_LAND_COMBAT, 5, 5);
        // Attaquant sur la même case, même classe : le partenaire ne contourne pas.
        assert!(excedent(&a, Some(&d(0, FORMATION_CLASS_LAND_COMBAT, 5, 5)), []));
        // Ailleurs, ou d'une autre classe : sans effet.
        assert!(!excedent(&a, Some(&d(0, FORMATION_CLASS_LAND_COMBAT, 6, 5)), []));
        assert!(!excedent(&a, Some(&d(0, FORMATION_CLASS_NAVAL, 5, 5)), []));
    }
}
