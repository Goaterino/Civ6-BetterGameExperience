//! Recherche de motifs d'octets dans la section `.text` de l'exe, avec jokers (`None`), et lecture
//! des cibles d'appels relatifs. Sert aux modules qui visent le DX12 et le DX11 sans table
//! d'adresses : le motif doit être unique (ou en nombre attendu) dans chaque exe.

use crate::exe;

/// Motif écrit comme dans un désassembleur : `"48 8B ?? 10"`.
pub fn motif(s: &str) -> Vec<Option<u8>> {
    s.split_whitespace().map(|t| if t == "??" { None } else { Some(u8::from_str_radix(t, 16).expect("octet du motif")) }).collect()
}

/// Positions (adresses) du motif dans `.text` du module.
pub unsafe fn chercher(module: usize, m: &[Option<u8>]) -> Vec<usize> {
    let Some((start, size)) = exe::text_section(module) else {
        return Vec::new();
    };
    let text = std::slice::from_raw_parts(start as *const u8, size);
    trouver(text, m).into_iter().map(|i| start + i).collect()
}

/// Positions du motif dans `hay`. Le premier octet fixe sert de filtre rapide.
pub fn trouver(hay: &[u8], m: &[Option<u8>]) -> Vec<usize> {
    let Some((ancre, &Some(octet))) = m.iter().enumerate().find(|(_, b)| b.is_some()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if hay.len() < m.len() {
        return out;
    }
    for i in 0..=hay.len() - m.len() {
        if hay[i + ancre] != octet {
            continue;
        }
        if m.iter().zip(&hay[i..i + m.len()]).all(|(p, h)| p.is_none_or(|p| p == *h)) {
            out.push(i);
        }
    }
    out
}

/// Cible d'un `call rel32` / `jmp rel32` (opcode en `at`).
pub unsafe fn cible_rel32(at: usize) -> usize {
    (at as i64 + 5 + exe::rd::<i32>(at + 1) as i64) as usize
}

/// Les octets en `at` correspondent-ils au motif ?
pub unsafe fn correspond(at: usize, m: &[Option<u8>]) -> bool {
    let have = std::slice::from_raw_parts(at as *const u8, m.len());
    m.iter().zip(have).all(|(p, h)| p.is_none_or(|p| p == *h))
}

/// Redirige le `call rel32` en `site` vers `hook`, par un saut absolu posé dans une page proche.
/// Rend l'ancienne cible.
pub unsafe fn rediriger_appel(site: usize, hook: usize) -> Result<usize, String> {
    if exe::rd::<u8>(site) != 0xe8 {
        return Err(format!("{site:#x} : pas un call rel32"));
    }
    let ancienne = cible_rel32(site);
    let page = crate::patch::alloc_near(site).ok_or("pas de page proche")?;
    let mut saut = [0u8; 14];
    saut[..6].copy_from_slice(&[0xff, 0x25, 0, 0, 0, 0]);
    saut[6..].copy_from_slice(&(hook as u64).to_le_bytes());
    std::ptr::copy_nonoverlapping(saut.as_ptr(), page as *mut u8, saut.len());
    let rel = crate::patch::rel32(site, page).ok_or("page trop loin")?;
    crate::patch::write_rel32(site, rel);
    Ok(ancienne)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jokers() {
        let hay = [0x90, 0x48, 0x8b, 0x05, 0x10, 0x48, 0x8b, 0x07, 0x10];
        assert_eq!(trouver(&hay, &motif("48 8B ?? 10")), vec![1, 5]);
        assert_eq!(trouver(&hay, &motif("?? 8B 07")), vec![5]);
        assert!(trouver(&hay, &motif("48 8B 05 11")).is_empty());
    }
}
