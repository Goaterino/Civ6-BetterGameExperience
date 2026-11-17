//! Conversion des pourcentages de production d'unités en 24.8, commune à Windows
//! (production_modifiers.rs) et macOS (production_mac.rs) : les deux doivent donner le même résultat,
//! au bit près, pour rester synchrones entre eux.

/// Opposé de la conversion positive du jeu : multiplication f32 par 0,01, puis
/// troncature en 24.8. Multiplier par 256 est exact. Hors plage, garder l'originale.
pub fn retrait(percent: i32) -> Option<i32> {
    let magnitude = -(percent as f32) * 0.01 * 256.0;
    (percent < 0 && magnitude < 2_147_483_648.0).then(|| -(magnitude as i32))
}

/// Conversion du jeu, instruction pour instruction : `x = p * 0,01f`, `floorf`, fraction × 256
/// tronquée sur un octet, partie entière décalée de 8.
pub fn conversion_jeu(percent: i32) -> i32 {
    let x = percent as f32 * 0.01;
    let entier = x.floor();
    (((x - entier) * 256.0) as i32 & 0xff) | (entier as i32).wrapping_shl(8)
}

/// Conversion sous POL : celle du jeu, sauf une variation négative qui dérive.
pub fn conversion(percent: i32) -> i32 {
    retrait(percent).unwrap_or_else(|| conversion_jeu(percent))
}

pub fn derive(percent: i32) -> bool {
    conversion(percent) != conversion_jeu(percent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrait_symetrique() {
        // Ajouts relevés dans la routine native, dont les valeurs exactement représentables.
        for (percent, added) in [(5, 12), (10, 25), (15, 38), (25, 64), (30, 76), (50, 128)] {
            assert_eq!(retrait(-percent), Some(-added));
        }
        assert_eq!(retrait(0), None);
        assert_eq!(retrait(30), None);
        assert_eq!(retrait(i32::MIN), None);
    }

    #[test]
    fn conversion_du_jeu() {
        // Valeurs de la routine native : arrondi vers -∞ pour un négatif.
        assert_eq!(conversion_jeu(30), 76);
        assert_eq!(conversion_jeu(-30), -77);
        assert_eq!(conversion_jeu(-50), -128);
        assert_eq!(conversion_jeu(150), 384);
        for p in 1..=100_000 {
            assert_eq!(conversion(p), conversion_jeu(p), "positif inchangé : {p}");
            assert_eq!(conversion(-p), -conversion_jeu(p), "retrait symétrique : {p}");
            assert_eq!(derive(-p), conversion_jeu(-p) != -conversion_jeu(p));
        }
        assert!(!derive(-25) && !derive(-50) && !derive(-100) && derive(-30) && !derive(30));
        assert_eq!(conversion(i32::MIN), conversion_jeu(i32::MIN));
    }

}
