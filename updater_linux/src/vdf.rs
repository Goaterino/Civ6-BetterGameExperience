//! Reader for Steam VDF files (`libraryfolders.vdf`, `appmanifest_*.acf`): a tree of `"key" "value"`
//! or `"key" { ... }` blocks. Quoted strings support the escapes `\\`, `\"`, `\n` and `\t`; `//`
//! comments run to the end of the line.

pub enum Value {
    Str(String),
    Block(Vec<(String, Value)>),
}

#[derive(PartialEq)]
enum Token {
    Str(String),
    Open,
    Close,
}

fn tokens(text: &str) -> Result<Vec<Token>, String> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            c if c.is_whitespace() => {
                chars.next();
            }
            '{' => {
                chars.next();
                out.push(Token::Open);
            }
            '}' => {
                chars.next();
                out.push(Token::Close);
            }
            '/' => {
                // Comment: up to the end of the line.
                while chars.next_if(|&c| c != '\n').is_some() {}
            }
            '"' => {
                chars.next();
                let mut s = String::new();
                loop {
                    match chars.next() {
                        None => return Err("chaîne non terminée".into()),
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(other) => s.push(other),
                            None => return Err("échappement non terminé".into()),
                        },
                        Some(c) => s.push(c),
                    }
                }
                out.push(Token::Str(s));
            }
            _ => {
                // Unquoted word.
                let mut s = String::new();
                while let Some(c) = chars.next_if(|c| !c.is_whitespace() && !matches!(c, '{' | '}' | '"')) {
                    s.push(c);
                }
                out.push(Token::Str(s));
            }
        }
    }
    Ok(out)
}

/// The contents of a VDF file: the sequence of its top-level pairs.
pub fn parse(text: &str) -> Result<Vec<(String, Value)>, String> {
    let tokens = tokens(text)?;
    let mut pos = 0;
    let entries = block(&tokens, &mut pos, true)?;
    Ok(entries)
}

fn block(tokens: &[Token], pos: &mut usize, top: bool) -> Result<Vec<(String, Value)>, String> {
    let mut entries = Vec::new();
    loop {
        match tokens.get(*pos) {
            None if top => return Ok(entries),
            None => return Err("bloc non fermé".into()),
            Some(Token::Close) if !top => {
                *pos += 1;
                return Ok(entries);
            }
            Some(Token::Close) => return Err("« } » en trop".into()),
            Some(Token::Open) => return Err("« { » sans clé".into()),
            Some(Token::Str(key)) => {
                *pos += 1;
                match tokens.get(*pos) {
                    Some(Token::Str(value)) => {
                        *pos += 1;
                        entries.push((key.clone(), Value::Str(value.clone())));
                    }
                    Some(Token::Open) => {
                        *pos += 1;
                        entries.push((key.clone(), Value::Block(block(tokens, pos, false)?)));
                    }
                    _ => return Err(format!("« {key} » sans valeur")),
                }
            }
        }
    }
}

/// The first value with key `key` (case-insensitive, like Steam).
pub fn get<'a>(entries: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    entries.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)
}

pub fn get_str<'a>(entries: &'a [(String, Value)], key: &str) -> Option<&'a str> {
    match get(entries, key)? {
        Value::Str(s) => Some(s),
        Value::Block(_) => None,
    }
}

pub fn get_block<'a>(entries: &'a [(String, Value)], key: &str) -> Option<&'a [(String, Value)]> {
    match get(entries, key)? {
        Value::Block(b) => Some(b),
        Value::Str(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libraryfolders() {
        let text = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"/home/u/.local/share/Steam\"\n\t\t\"apps\"\n\t\t{\n\t\t\t\"228980\"\t\t\"403678624\"\n\t\t}\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"/mnt/games/Steam \\\"Library\\\" \\\\x\"\n\t}\n}\n";
        let root = parse(text).unwrap();
        let folders = get_block(&root, "libraryfolders").unwrap();
        let first = get_block(folders, "0").unwrap();
        assert_eq!(get_str(first, "path"), Some("/home/u/.local/share/Steam"));
        assert_eq!(get_str(get_block(first, "apps").unwrap(), "228980"), Some("403678624"));
        assert_eq!(get_str(get_block(folders, "1").unwrap(), "path"), Some("/mnt/games/Steam \"Library\" \\x"));
    }

    #[test]
    fn commentaires_et_mots_nus() {
        let root = parse("// début\n\"a\" b // fin\n\"c\" { d \"e f\" }\n").unwrap();
        assert_eq!(get_str(&root, "a"), Some("b"));
        assert_eq!(get_str(get_block(&root, "c").unwrap(), "d"), Some("e f"));
    }

    #[test]
    fn erreurs() {
        assert!(parse("\"a\" {").is_err());
        assert!(parse("}").is_err());
        assert!(parse("\"a\" \"b").is_err());
        assert!(parse("\"a\"").is_err());
    }
}
