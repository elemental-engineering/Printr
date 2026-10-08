//! Horoscope hors ligne et gratuit, calculé sur le ciel réel par Barnum
//! (https://github.com/XNinety9/Barnum), appelé en ligne de commande avec `--json`.
//!
//! Commande : `$PRINTR_BARNUM` (par exemple `python3 /opt/barnum/main.py`), sinon `barnum`.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
use serde::Deserialize;

use crate::doc::{Doc, Style};
use crate::fr;

const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
struct Reading {
    signe: String,
    #[serde(default)]
    lune: Option<Moon>,
    #[serde(default)]
    climat: String,
    #[serde(default)]
    domaines: Vec<Domain>,
    #[serde(default)]
    conseil: String,
    #[serde(default)]
    couleur: String,
    #[serde(default)]
    chiffre: Option<i64>,
    #[serde(default)]
    humeur: String,
    #[serde(default)]
    ciel: Vec<Body>,
}

#[derive(Deserialize)]
struct Moon {
    phase: String,
    signe: String,
}

#[derive(Deserialize)]
struct Domain {
    cle: String,
    note: u8,
    texte: String,
}

#[derive(Deserialize)]
struct Body {
    astre: String,
    signe: String,
    degre: f64,
    #[serde(default)]
    retrograde: bool,
}

/// Commande de Barnum, découpée en programme et arguments.
pub fn command() -> Vec<String> {
    std::env::var("PRINTR_BARNUM")
        .ok()
        .filter(|c| !c.trim().is_empty())
        .unwrap_or_else(|| "barnum".to_owned())
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// Lance Barnum et lit sa réponse JSON.
fn run(command: &[String], who: &Who, date: NaiveDate, variant: Option<&str>) -> Result<Reading> {
    let (program, base) = command.split_first().context("commande Barnum vide")?;
    let mut cmd = Command::new(program);
    cmd.args(base);
    match who {
        Who::Sign(sign) => cmd.arg(sign),
        Who::Birth(birth) => cmd.arg("--naissance").arg(birth.to_string()),
    };
    cmd.arg("--date").arg(date.to_string()).arg("--json");
    if let Some(variant) = variant {
        cmd.arg("--sel").arg(variant);
    }
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => anyhow::anyhow!(
                "Barnum introuvable (« {program} ») : installe-le ou définis PRINTR_BARNUM"
            ),
            _ => anyhow::anyhow!("impossible de lancer Barnum : {e}"),
        })?;

    // Barnum répond en quelques dizaines de millisecondes ; au-delà de TIMEOUT, on abandonne.
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() > TIMEOUT {
            let _ = child.kill();
            bail!("Barnum ne répond pas");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let mut stdout = String::new();
    let mut stderr = String::new();
    child.stdout.take().map(|mut s| s.read_to_string(&mut stdout));
    child.stderr.take().map(|mut s| s.read_to_string(&mut stderr));
    if !status.success() {
        // « barnum: error: signe inconnu : … » : on garde le message utile.
        let message = stderr.lines().last().unwrap_or("erreur inconnue");
        bail!("{}", message.trim_start_matches("barnum: error: "));
    }
    serde_json::from_str(&stdout).context("réponse de Barnum illisible")
}

pub enum Who {
    Sign(String),
    Birth(NaiveDate),
}

/// Jauge de note sur 5 : « ███░░ ».
fn gauge(note: u8) -> String {
    let note = note.min(5) as usize;
    format!("{}{}", "█".repeat(note), "░".repeat(5 - note))
}

fn render(r: &Reading, sky: bool) -> Doc {
    let mut doc = Doc::new();
    doc.header(&format!("Horoscope · {}", r.signe));
    if let Some(moon) = &r.lune {
        doc.text(&format!("Lune : {} en {}", moon.phase, moon.signe), Style::default().small().center());
    }
    doc.feed(1);
    doc.text(&r.climat, Style::default());
    for d in &r.domaines {
        doc.feed(1);
        doc.line(&format!("{:<14}{}", fr::capitalize(&d.cle), gauge(d.note)), Style::default().bold());
        doc.text(&d.texte, Style::default());
    }
    if !r.conseil.is_empty() {
        doc.feed(1);
        doc.hanging("Conseil : ", &r.conseil, Style::default().bold());
    }
    let mut extras = Vec::new();
    if !r.couleur.is_empty() {
        extras.push(format!("couleur {}", r.couleur));
    }
    if let Some(n) = r.chiffre {
        extras.push(format!("chiffre {n}"));
    }
    if !r.humeur.is_empty() {
        extras.push(format!("humeur {}", r.humeur));
    }
    if !extras.is_empty() {
        doc.text(&fr::capitalize(&extras.join(" · ")), Style::default().small().center());
    }
    if sky && !r.ciel.is_empty() {
        doc.feed(1);
        doc.text("Le ciel du jour", Style::default().small().bold().center());
        for b in &r.ciel {
            // Même largeur pour toutes les lignes : les colonnes restent alignées une fois centrées.
            let retro = if b.retrograde { "rétrograde" } else { "" };
            let line = format!("{:<9}{:<12}{:>3.0}°  {retro:<10}", b.astre, b.signe, b.degre);
            doc.line(&line, Style::default().small().center());
        }
    }
    doc
}

pub fn build(who: Who, date: NaiveDate, sky: bool, variant: Option<&str>) -> Result<Doc> {
    let reading = run(&command(), &who, date, variant)?;
    Ok(render(&reading, sky))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "signe": "Lion", "date": "2026-10-08",
        "lune": {"phase": "dernier croissant", "signe": "Vierge"},
        "transit": {"astre": "Mars", "aspect": "conjonction"},
        "climat": "Mars traverse votre signe et vous pousse à brûler les étapes.",
        "domaines": [
            {"cle": "amour", "note": 1, "texte": "Vénus vous complique la tâche.", "influence": "Vénus"},
            {"cle": "argent", "note": 4, "texte": "Bonne journée pour faire vos comptes.", "influence": null}
        ],
        "conseil": "La simplicité est une force.", "couleur": "violet", "chiffre": 20, "humeur": "joueuse",
        "ciel": [{"astre": "Vénus", "signe": "Scorpion", "degre": 7.95, "retrograde": true}]
    }"#;

    fn lines(doc: &Doc) -> Vec<String> {
        doc.ops
            .iter()
            .filter_map(|op| match op {
                crate::doc::Op::Line { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn renders_reading_with_gauges_and_sky() {
        let reading: Reading = serde_json::from_str(SAMPLE).unwrap();
        let all = lines(&render(&reading, true)).join("\n");
        assert!(all.contains("Amour         █░░░░"));
        assert!(all.contains("Argent        ████░"));
        assert!(all.contains("Couleur violet · chiffre 20 · humeur joueuse"));
        assert!(all.contains("rétrograde"));
        assert!(!lines(&render(&reading, false)).join("\n").contains("rétrograde"));
    }

    #[test]
    fn missing_program_is_explained() {
        let cmd = vec!["printr-barnum-introuvable".to_owned()];
        let date = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        let err = run(&cmd, &Who::Sign("lion".into()), date, None).err().unwrap();
        assert!(err.to_string().contains("Barnum introuvable"));
    }
}
