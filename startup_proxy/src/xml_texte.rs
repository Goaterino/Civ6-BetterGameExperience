//! Repérage dans les XML de l'interface, commun à Windows et macOS.

/// Fin du contexte : la dernière balise fermante `</Context>` ou `</ContextDefaults>`.
pub(crate) fn fin_du_contexte(text: &str) -> Result<usize, String> {
    ["</ContextDefaults>", "</Context>"].iter().filter_map(|f| text.rfind(f)).max().ok_or_else(|| "fin du contexte introuvable".to_string())
}

/// Début et fin de l'élément qui porte `ancre` dans sa balise ouvrante, enfants compris.
pub(crate) fn element(text: &str, ancre: &str) -> Option<(usize, usize)> {
    let at = text.find(ancre)?;
    let debut = text[..at].rfind('<')?;
    let nom: String = text[debut + 1..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
    if nom.is_empty() {
        return None;
    }
    let fin_balise = text[at..].find('>')? + at + 1;
    if text[..fin_balise].ends_with("/>") {
        return Some((debut, fin_balise));
    }
    // Enfants : on cherche la fermeture du même niveau.
    let (ouvre, ferme) = (format!("<{nom}"), format!("</{nom}>"));
    let mut profondeur = 1;
    let mut i = fin_balise;
    while profondeur > 0 {
        let o = text[i..].find(&ouvre).map(|p| p + i);
        let f = text[i..].find(&ferme).map(|p| p + i)?;
        match o {
            Some(o) if o < f => {
                let fin_o = text[o..].find('>')? + o + 1;
                if !text[..fin_o].ends_with("/>") {
                    profondeur += 1;
                }
                i = fin_o;
            }
            _ => {
                profondeur -= 1;
                i = f + ferme.len();
            }
        }
    }
    Some((debut, i))
}
