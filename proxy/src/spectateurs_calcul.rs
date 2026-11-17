//! Tourisme sans les spectateurs : la formule du jeu, commune à Windows (spectateurs.rs) et macOS
//! (spectateurs_mac.rs), pour un résultat identique au bit près.

/// Touristes que A prend à P (`18027ce10`), entiers comme le jeu : `tourisme_vers` est le tourisme
/// cumulé de A vers P et `culture_p` la culture cumulée de P, tous deux en virgule fixe 24.8.
/// `demande` est la somme des mêmes quotients sur tous les majeurs qui visent P (`demande`).
pub fn touristes(tourisme_vers: i32, culture_p: i32, demande: i32, par_citoyen: i32, par_touriste: i32, n: i32) -> i32 {
    let domestiques = (culture_p >> 8).wrapping_div(par_citoyen);
    let t = quotient(tourisme_vers, par_touriste, n);
    if domestiques < demande {
        // Plafond : les domestiques de P, répartis au prorata (au moins 1 au dénominateur).
        let d = if demande >= 1 { demande } else { 1 };
        return (t.wrapping_mul(domestiques) as i64 / d as i64) as i32;
    }
    t
}

/// La part d'un visiteur dans la demande : `(tourisme >> 8) / par_touriste / n`, divisions entières.
pub fn quotient(tourisme_vers: i32, par_touriste: i32, n: i32) -> i32 {
    (tourisme_vers >> 8).wrapping_div(par_touriste).wrapping_div(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La formule du jeu, transcrite de l'assembleur de 18027ce10 (idiv 32 bits, imul 32 bits).
    fn jeu(vers: i32, culture: i32, demande: i32, par_citoyen: i32, par_touriste: i32, n: i32) -> i32 {
        let ebp = (culture >> 8) / par_citoyen;
        let eax = ((vers >> 8) / par_touriste) / n;
        if ebp >= demande {
            return eax;
        }
        let d = if demande >= 1 { demande } else { 1 };
        eax.wrapping_mul(ebp) / d
    }

    #[test]
    fn meme_formule_que_le_jeu() {
        let mut x: u64 = 0x2545_f491_4f6c_dd1d;
        let mut alea = |m: i64| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x % m as u64) as i32
        };
        for _ in 0..100_000 {
            let (vers, culture) = (alea(1 << 28) - (1 << 20), alea(1 << 28));
            let (demande, n) = (alea(5000) - 10, alea(12) + 1);
            assert_eq!(touristes(vers, culture, demande, 100, 200, n), jeu(vers, culture, demande, 100, 200, n));
        }
    }

    #[test]
    fn exemples() {
        // 12 000 de tourisme vers P, 6 majeurs : 12000 / 200 / 6 = 10 touristes.
        assert_eq!(touristes(12_000 << 8, 5_000 << 8, 10, 100, 200, 6), 10);
        // Avec 2 spectateurs en plus, le jeu divise par 8 : 7.
        assert_eq!(touristes(12_000 << 8, 5_000 << 8, 10, 100, 200, 8), 7);
        // Plafond : P n'a que 20 domestiques (2 000 de culture) pour 40 demandés.
        assert_eq!(touristes(12_000 << 8, 2_000 << 8, 40, 100, 200, 6), 5);
        // Spectateur visé : aucune culture, donc rien à prendre.
        assert_eq!(touristes(12_000 << 8, 0, 10, 100, 200, 6), 0);
    }
}
