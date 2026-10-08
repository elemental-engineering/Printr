//! Sortie console : rapports de construction, résumé, erreurs, journal du serveur.
//! Tout passe par `anstream`, qui retire les couleurs hors terminal ou avec `NO_COLOR`.

use std::io::{IsTerminal, Write};
use std::time::{Duration, Instant};

use anstream::eprintln;
use anstyle::{AnsiColor, Style};

use crate::blocks::{Progress, Report};
use crate::doc::Doc;

const OK: Style = AnsiColor::Green.on_default().bold();
const FAIL: Style = AnsiColor::Red.on_default().bold();
const WARN: Style = AnsiColor::Yellow.on_default().bold();
const ACCENT: Style = AnsiColor::Cyan.on_default();
const DIM: Style = Style::new().dimmed();
const BOLD: Style = Style::new().bold();

/// « 312 ms », « 1,4 s ».
pub fn duration(d: Duration) -> String {
    if d < Duration::from_secs(1) {
        format!("{} ms", d.as_millis())
    } else {
        format!("{:.1} s", d.as_secs_f64()).replace('.', ",")
    }
}

/// Longueur de papier lisible : « 12 cm », « 1,2 m ».
fn paper_length(dots: u32) -> String {
    let cm = dots as f64 / 180.0 * 2.54;
    if cm < 100.0 {
        format!("{cm:.0} cm")
    } else {
        format!("{:.1} m", cm / 100.0).replace('.', ",")
    }
}

fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n > 1 { "s" } else { "" })
}

/// Ligne d'un bloc terminé : statut, nom, durée ou erreur.
fn report_line(r: &Report, width: usize) -> String {
    let label: String = r.label.chars().take(width).collect();
    let pad = " ".repeat(width - label.chars().count());
    match &r.error {
        Some(error) => format!("  {FAIL}✗{FAIL:#} {label}{pad}  {FAIL}{error}{FAIL:#}"),
        None if r.cached => format!("  {OK}✓{OK:#} {label}{pad}  {DIM}cache{DIM:#}"),
        None if r.elapsed >= Duration::from_millis(50) => {
            format!("  {OK}✓{OK:#} {label}{pad}  {DIM}{}{DIM:#}", duration(r.elapsed))
        }
        None => format!("  {OK}✓{OK:#} {label}"),
    }
}

const SPINNER: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Affichage de la construction en console : chaque bloc s'affiche dès qu'il est prêt et,
/// dans un terminal, une ligne d'état animée liste les blocs encore en cours.
pub struct Console {
    /// Ligne d'état animée (seulement si la sortie d'erreur est un terminal).
    live: bool,
    color: bool,
    width: usize,
    labels: Vec<String>,
    pending: Vec<bool>,
    started: Instant,
    frame: usize,
    status_shown: bool,
}

impl Console {
    pub fn new() -> Self {
        let stderr = std::io::stderr();
        Self {
            live: stderr.is_terminal(),
            color: anstream::AutoStream::choice(&stderr) != anstream::ColorChoice::Never,
            width: 0,
            labels: Vec::new(),
            pending: Vec::new(),
            started: Instant::now(),
            frame: 0,
            status_shown: false,
        }
    }

    /// Écrit tel quel sur la sortie d'erreur (les codes de curseur ne passent pas par
    /// `anstream`, qui les retirerait avec `NO_COLOR`) ; les couleurs sont retirées au besoin.
    fn write(&self, text: &str) {
        let text = if self.color { text.to_owned() } else { anstream::adapter::strip_str(text).to_string() };
        let mut err = std::io::stderr().lock();
        let _ = err.write_all(text.as_bytes());
        let _ = err.flush();
    }

    fn clear_status(&mut self) {
        if self.status_shown {
            self.write("\r\x1b[2K");
            self.status_shown = false;
        }
    }

    fn draw_status(&mut self) {
        if !self.live {
            return;
        }
        let pending: Vec<&str> =
            self.labels.iter().zip(&self.pending).filter(|(_, p)| **p).map(|(l, _)| l.as_str()).collect();
        if pending.is_empty() {
            return self.clear_status();
        }
        let spinner = SPINNER[self.frame % SPINNER.len()];
        let text = format!("en cours ({}) · {} · {}", pending.len(), duration(self.started.elapsed()), pending.join(", "));
        // La ligne ne doit jamais déborder, sinon l'effacement laisserait des restes.
        let columns = terminal_size::terminal_size().map_or(80, |(w, _)| w.0 as usize);
        let max = columns.saturating_sub(5);
        let text = if text.chars().count() > max {
            format!("{}…", text.chars().take(max.saturating_sub(1)).collect::<String>())
        } else {
            text
        };
        self.write(&format!("\r\x1b[2K  {ACCENT}{spinner}{ACCENT:#} {DIM}{text}{DIM:#}"));
        self.status_shown = true;
    }
}

impl Progress for Console {
    fn start(&mut self, labels: &[String]) {
        self.labels = labels.to_vec();
        self.pending = vec![true; labels.len()];
        self.width = labels.iter().map(|l| l.chars().count()).max().unwrap_or(0).min(36);
        self.started = Instant::now();
        self.draw_status();
    }

    fn tick(&mut self) {
        self.frame += 1;
        self.draw_status();
    }

    fn done(&mut self, index: usize, report: &Report) {
        self.clear_status();
        self.pending[index] = false;
        self.write(&format!("{}\n", report_line(report, self.width)));
        self.draw_status();
    }

    fn finish(&mut self) {
        self.clear_status();
    }
}

/// Résumé final : destination, nombre de blocs, longueur de papier.
pub fn summary(doc: &Doc, blocks: usize, destination: Option<&str>, elapsed: Duration, failures: usize) {
    let mut details = Vec::new();
    if blocks > 0 {
        details.push(plural(blocks, "bloc"));
    }
    details.push(format!("≈ {} de papier", paper_length(doc.height_dots())));
    if failures > 0 {
        details.push(format!("{WARN}{} en échec{WARN:#}", failures));
    }
    details.push(duration(elapsed));
    let details = details.join(&format!(" {DIM}·{DIM:#} "));
    let mark = if failures > 0 { format!("{WARN}!{WARN:#}") } else { format!("{OK}✓{OK:#}") };
    match destination {
        Some(dest) => eprintln!("{mark} Imprimé sur {ACCENT}{dest}{ACCENT:#}  {DIM}│{DIM:#} {details}"),
        None => eprintln!("{mark} Aperçu  {DIM}│{DIM:#} {details}"),
    }
}

/// Erreur fatale, avec la chaîne des causes.
pub fn error(e: &anyhow::Error) {
    eprintln!("{FAIL}✗ Erreur :{FAIL:#} {BOLD}{e}{BOLD:#}");
    for cause in e.chain().skip(1) {
        eprintln!("  {DIM}↳{DIM:#} {cause}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(duration(Duration::from_millis(312)), "312 ms");
        assert_eq!(duration(Duration::from_millis(1450)), "1,4 s");
        assert_eq!(paper_length(180), "3 cm");
        assert_eq!(paper_length(7200), "1,0 m");
        assert_eq!(plural(1, "bloc"), "1 bloc");
        assert_eq!(plural(3, "bloc"), "3 blocs");
    }
}
