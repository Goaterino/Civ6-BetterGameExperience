//! Retour au vote du congrès : la règle d'annulation de fin de tour, commune à Windows (congres.rs)
//! et macOS (congres_mac.rs), pour que toutes les machines décident de la même façon.

/// Hachages des segments de vote (`~CRC32`, comme `DB.MakeHash`).
pub const WC_1: u32 = 0xb52a_667b;
pub const WC_2: u32 = 0x2c23_37c1;

/// Annulation de fin de tour permise : le drapeau `AllowTurnUnready` du segment, levé pendant le
/// vote si le correctif est actif.
pub fn decision(drapeau: bool, type_segment: u32, actif: bool) -> bool {
    drapeau || (actif && (type_segment == WC_1 || type_segment == WC_2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annulation_pendant_le_vote_seulement() {
        // Segment normal : le drapeau du jeu, actif ou non.
        assert!(decision(true, 0x1234, false));
        assert!(!decision(false, 0x1234, true));
        // Vote : permis si le correctif est actif.
        assert!(decision(false, WC_1, true));
        assert!(decision(false, WC_2, true));
        assert!(!decision(false, WC_1, false));
        // Résultats (TURNSEG_WORLDCONGRESS_RESOLUTION) : jamais.
        assert!(!decision(false, 0xd74f_83e5, true));
    }

    #[test]
    fn hachages_des_segments() {
        let h = |s: &str| !crc32(s.as_bytes());
        assert_eq!(h("TURNSEG_WORLDCONGRESS_1"), WC_1);
        assert_eq!(h("TURNSEG_WORLDCONGRESS_2"), WC_2);
    }

    fn crc32(b: &[u8]) -> u32 {
        let mut c = !0u32;
        for &x in b {
            c ^= x as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 };
            }
        }
        !c
    }
}
