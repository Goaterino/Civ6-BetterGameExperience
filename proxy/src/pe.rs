#![allow(dead_code)]
//! Lecture d'une image PE x64, chargée en mémoire ou lue d'un fichier (tests), pour trouver un site
//! sans table d'adresses : une chaîne, les `lea` qui la désignent, la fonction qui les contient
//! (table des exceptions, entrées enchaînées comprises), les `call`/`jmp rel32` vers une fonction et
//! les tables virtuelles qui la contiennent. Commun au proxy de GameCore et au proxy de démarrage.

/// Une image : mappée (adresses relatives = décalages) ou fichier (décalages par la table des
/// sections).
pub struct Image<'a> {
    bytes: &'a [u8],
    mapped: bool,
    /// Adresse où l'image est chargée (mappée), ou son adresse préférée (fichier) : celle des
    /// pointeurs absolus qu'elle contient.
    pub base: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct Section {
    pub name: [u8; 8],
    pub rva: u32,
    pub size: u32,
    raw: u32,
    raw_size: u32,
}

fn u16_at(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

fn u64_at(b: &[u8], o: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(o..o + 8)?.try_into().ok()?))
}

impl<'a> Image<'a> {
    /// Fichier PE lu en entier.
    pub fn file(bytes: &'a [u8]) -> Option<Self> {
        let mut img = Image { bytes, mapped: false, base: 0 };
        let nt = u32_at(bytes, 0x3c)? as usize;
        img.base = u64_at(bytes, nt + 24 + 24)?;
        Some(img)
    }

    /// Module chargé à `base`.
    ///
    /// # Safety
    /// `base` est un module chargé, qui reste chargé tant que l'image sert.
    pub unsafe fn mapped(base: usize) -> Option<Self> {
        let nt = std::ptr::read_unaligned((base + 0x3c) as *const u32) as usize;
        let size = std::ptr::read_unaligned((base + nt + 24 + 56) as *const u32) as usize;
        Some(Image { bytes: std::slice::from_raw_parts(base as *const u8, size), mapped: true, base: base as u64 })
    }

    fn nt(&self) -> usize {
        u32_at(self.bytes, 0x3c).unwrap_or(0) as usize
    }

    pub fn sections(&self) -> Vec<Section> {
        let nt = self.nt();
        let (Some(n), Some(opt)) = (u16_at(self.bytes, nt + 6), u16_at(self.bytes, nt + 20)) else { return Vec::new() };
        let first = nt + 24 + opt as usize;
        (0..n as usize)
            .filter_map(|i| {
                let s = first + 40 * i;
                Some(Section {
                    name: self.bytes.get(s..s + 8)?.try_into().ok()?,
                    size: u32_at(self.bytes, s + 8)?,
                    rva: u32_at(self.bytes, s + 12)?,
                    raw_size: u32_at(self.bytes, s + 16)?,
                    raw: u32_at(self.bytes, s + 20)?,
                })
            })
            .collect()
    }

    pub fn section(&self, name: &str) -> Option<Section> {
        self.sections().into_iter().find(|s| s.name.split(|&b| b == 0).next() == Some(name.as_bytes()))
    }

    /// Octets de la section (taille virtuelle, bornée par les données du fichier).
    pub fn data(&self, s: &Section) -> &'a [u8] {
        let (start, len) = if self.mapped {
            (s.rva as usize, s.size as usize)
        } else {
            (s.raw as usize, s.size.min(s.raw_size) as usize)
        };
        self.bytes.get(start..start + len).unwrap_or(&[])
    }

    /// `n` octets à l'adresse relative `rva`, dans une section.
    pub fn at(&self, rva: u32, n: usize) -> Option<&'a [u8]> {
        if self.mapped {
            return self.bytes.get(rva as usize..rva as usize + n);
        }
        let s = self.sections().into_iter().find(|s| rva >= s.rva && rva < s.rva + s.size.max(s.raw_size))?;
        let o = (rva - s.rva + s.raw) as usize;
        self.bytes.get(o..o + n)
    }

    /// Adresses relatives de `needle` dans la section `name`.
    pub fn find(&self, name: &str, needle: &[u8]) -> Vec<u32> {
        let Some(s) = self.section(name) else { return Vec::new() };
        let data = self.data(&s);
        if needle.is_empty() || data.len() < needle.len() {
            return Vec::new();
        }
        (0..=data.len() - needle.len()).filter(|&i| &data[i..i + needle.len()] == needle).map(|i| s.rva + i as u32).collect()
    }

    /// Les `lea r64,[rip+disp32]` de `.text` qui désignent `target`.
    pub fn lea_refs(&self, target: u32) -> Vec<u32> {
        let Some(s) = self.section(".text") else { return Vec::new() };
        let t = self.data(&s);
        (0..t.len().saturating_sub(7))
            .filter(|&i| {
                (t[i] == 0x48 || t[i] == 0x4c)
                    && t[i + 1] == 0x8d
                    && t[i + 2] & 0xc7 == 0x05
                    && (s.rva as i64 + i as i64 + 7 + i32::from_le_bytes(t[i + 3..i + 7].try_into().unwrap()) as i64) == target as i64
            })
            .map(|i| s.rva + i as u32)
            .collect()
    }

    /// Début de la fonction qui contient `rva`, selon la table des exceptions : une entrée
    /// enchaînée (morceau de fonction) renvoie à la fonction principale.
    pub fn function_start(&self, rva: u32) -> Option<u32> {
        let nt = self.nt();
        // Répertoire 3 (exceptions) de l'en-tête optionnel PE32+.
        let dir = nt + 24 + 112 + 3 * 8;
        let (table, size) = (u32_at(self.bytes, dir)?, u32_at(self.bytes, dir + 4)?);
        let entries = self.at(table, size as usize)?;
        let entry = |k: usize| -> Option<(u32, u32, u32)> { Some((u32_at(entries, 12 * k)?, u32_at(entries, 12 * k + 4)?, u32_at(entries, 12 * k + 8)?)) };
        let n = size as usize / 12;
        // Entrées triées par adresse de début.
        let (mut lo, mut hi) = (0usize, n);
        while lo < hi {
            let mid = (lo + hi) / 2;
            if entry(mid)?.0 <= rva {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let (mut begin, end, mut unwind) = entry(lo.checked_sub(1)?)?;
        if rva >= end {
            return None;
        }
        for _ in 0..32 {
            let info = self.at(unwind, 4)?;
            // UNW_FLAG_CHAININFO : l'entrée principale suit les codes de déroulement.
            if info[0] >> 3 & 4 == 0 {
                return Some(begin);
            }
            let codes = (info[2] as u32 + 1) & !1;
            let chained = self.at(unwind + 4 + 2 * codes, 12)?;
            begin = u32_at(chained, 0)?;
            unwind = u32_at(chained, 8)?;
        }
        None
    }

    /// Les `call rel32` (E8) et `jmp rel32` (E9) de `.text` vers `target` : (adresse relative,
    /// opcode).
    pub fn rel32_sites(&self, target: u32) -> Vec<(u32, u8)> {
        let Some(s) = self.section(".text") else { return Vec::new() };
        let t = self.data(&s);
        (0..t.len().saturating_sub(5))
            .filter(|&i| {
                (t[i] == 0xe8 || t[i] == 0xe9)
                    && s.rva as i64 + i as i64 + 5 + i32::from_le_bytes(t[i + 1..i + 5].try_into().unwrap()) as i64 == target as i64
            })
            .map(|i| (s.rva + i as u32, t[i]))
            .collect()
    }

    /// Cible relative d'un `call`/`jmp rel32` en `site`.
    pub fn rel32_target(&self, site: u32) -> Option<u32> {
        let b = self.at(site, 5)?;
        Some((site as i64 + 5 + i32::from_le_bytes(b[1..5].try_into().ok()?) as i64) as u32)
    }

    /// Les qwords alignés de la section `name` qui valent le pointeur absolu de `target`.
    pub fn pointers_to(&self, name: &str, target: u32) -> Vec<u32> {
        let Some(s) = self.section(name) else { return Vec::new() };
        let d = self.data(&s);
        let value = self.base + target as u64;
        (0..d.len() / 8).filter(|&k| u64_at(d, 8 * k) == Some(value)).map(|k| s.rva + 8 * k as u32).collect()
    }

    /// Pointeur absolu rangé en `rva`, rendu en adresse relative.
    pub fn pointer_at(&self, rva: u32) -> Option<u32> {
        let p = u64_at(self.at(rva, 8)?, 0)?;
        p.checked_sub(self.base).and_then(|r| u32::try_from(r).ok())
    }

    /// La fonction unique dont le code désigne la chaîne `text` (terminée par un zéro) de `.rdata`.
    pub fn function_of_string(&self, text: &[u8]) -> Result<u32, String> {
        let mut needle = text.to_vec();
        needle.push(0);
        let strings = self.find(".rdata", &needle);
        let [string] = strings[..] else {
            return Err(format!("« {} » : {} occurrences au lieu de 1", String::from_utf8_lossy(text), strings.len()));
        };
        let refs = self.lea_refs(string);
        let mut functions: Vec<u32> = refs.iter().filter_map(|&r| self.function_start(r)).collect();
        functions.sort_unstable();
        functions.dedup();
        match (refs.len(), &functions[..]) {
            (0, _) => Err(format!("« {} » : aucune référence", String::from_utf8_lossy(text))),
            (n, [f]) if refs.iter().all(|&r| self.function_start(r) == Some(*f)) && n > 0 => Ok(*f),
            (_, fs) => Err(format!("« {} » : {} fonctions la désignent", String::from_utf8_lossy(text), fs.len())),
        }
    }
}

/// Motif d'octets avec jokers (`??`), comme dans un désassembleur.
pub fn pattern(s: &str) -> Vec<Option<u8>> {
    s.split_whitespace().map(|t| if t == "??" { None } else { u8::from_str_radix(t, 16).ok() }).collect()
}

/// Positions du motif dans `hay`.
pub fn find_pattern(hay: &[u8], m: &[Option<u8>]) -> Vec<usize> {
    if m.is_empty() || hay.len() < m.len() {
        return Vec::new();
    }
    (0..=hay.len() - m.len()).filter(|&i| m.iter().zip(&hay[i..]).all(|(p, b)| p.is_none_or(|p| p == *b))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jokers() {
        let hay = [0x90, 0x48, 0x8b, 0x05, 0x10, 0x48, 0x8b, 0x07, 0x10];
        assert_eq!(find_pattern(&hay, &pattern("48 8B ?? 10")), vec![1, 5]);
        assert!(find_pattern(&hay, &pattern("48 8B 05 11")).is_empty());
    }
}
