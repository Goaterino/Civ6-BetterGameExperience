//! Command-line interface, in place of `gui.rs` (TaskDialog): everything goes through stdout, stderr
//! and stdin. Without a terminal (launched by the game), nothing is asked: `main.rs` only calls
//! `choose` and `confirm` when `interactive()`.

use std::io::{BufRead, IsTerminal, Write};

#[derive(Clone, Copy)]
pub enum Icon {
    Info,
    Warning,
    Error,
}

/// True if stdin and stdout are a terminal: questions can be asked.
pub fn interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

/// Message: errors and warnings on stderr, the rest on stdout.
pub fn message(icon: Icon, title: &str, body: &str) {
    let text = if body.is_empty() { title.to_string() } else { format!("{title}\n{body}") };
    match icon {
        Icon::Info => println!("{text}"),
        Icon::Warning => eprintln!("[!] {text}"),
        Icon::Error => eprintln!("[x] {text}"),
    }
}

/// Runs `work` after announcing `label` (no progress bar: one line is enough).
pub fn progress<T>(label: &str, work: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    println!("{label}…");
    work()
}

/// Numbered menu: the chosen number (from 0), or `None` to quit (empty line, end of file or number
/// outside the list).
pub fn choose(prompt: &str, options: &[&str]) -> Option<usize> {
    for (i, option) in options.iter().enumerate() {
        println!("  {}. {option}", i + 1);
    }
    print!("{prompt} ");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line).ok()?;
    let n: usize = line.trim().parse().ok()?;
    (1..=options.len()).contains(&n).then(|| n - 1)
}
