//! Blocs paramétrables d'un ticket, décrits en JSON.

mod anagram;
pub mod barnum;
mod bins;
mod cipher;
mod coloring;
mod glitch;
mod holidays;
mod maze;
mod mental_math;
mod moon;
mod nonogram;
mod petit_bac;
mod picto;
mod quote;
mod riddle;
mod saint;
mod sudoku;
mod train_tracks;
mod word_search;
mod wifi;

use std::panic::AssertUnwindSafe;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::NaiveDate;
use rand::RngExt;
use serde::Deserialize;

use crate::doc::{Align, Doc, Style};
use crate::{fr, raster};

fn yes() -> bool {
    true
}
fn one() -> u8 {
    1
}
fn two() -> u8 {
    2
}
fn three() -> u8 {
    3
}
fn six() -> u8 {
    6
}
fn ten() -> u8 {
    10
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ticket {
    /// Coupe le papier à la fin.
    #[serde(default = "yes")]
    pub cut: bool,
    /// Lignes vides entre deux blocs.
    #[serde(default = "one")]
    pub spacing: u8,
    pub blocks: Vec<Block>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Block {
    Title {
        text: String,
        #[serde(default = "two")]
        size: u8,
    },
    Text {
        text: String,
        #[serde(default)]
        bold: bool,
        #[serde(default)]
        underline: bool,
        #[serde(default)]
        reverse: bool,
        #[serde(default)]
        small: bool,
        #[serde(default = "one")]
        size: u8,
        #[serde(default)]
        align: Align,
    },
    /// Message étrange de la machine (ajouté aussi au hasard, voir `glitch::chance`).
    Glitch {
        #[serde(default)]
        seed: Option<u64>,
    },
    Separator {
        #[serde(default)]
        style: Option<char>,
    },
    Date {},
    Feed {
        #[serde(default = "one")]
        lines: u8,
    },
    /// Image tirée d'un fichier local (logo, dessin…).
    Image {
        path: String,
        #[serde(default = "yes")]
        dither: bool,
    },
    Qr {
        data: String,
        #[serde(default)]
        size: Option<u8>,
        #[serde(default)]
        caption: Option<String>,
    },
    Saint {},
    Todo {
        #[serde(default)]
        title: Option<String>,
        items: Vec<String>,
    },
    Sudoku {
        #[serde(default)]
        difficulty: sudoku::Difficulty,
        #[serde(default)]
        seed: Option<u64>,
        #[serde(default)]
        solution: bool,
    },
    #[serde(alias = "mots_meles", alias = "mots_caches", alias = "mots_en_grille")]
    WordSearch {
        #[serde(default)]
        difficulty: sudoku::Difficulty,
        /// Thème des mots ; tiré du numéro de grille si absent.
        #[serde(default)]
        theme: Option<word_search::Theme>,
        /// Mots à cacher à la place d'un thème.
        #[serde(default)]
        words: Vec<String>,
        #[serde(default)]
        seed: Option<u64>,
        #[serde(default)]
        solution: bool,
    },
    #[serde(alias = "voie_ferree", alias = "rails")]
    TrainTracks {
        #[serde(default)]
        difficulty: sudoku::Difficulty,
        #[serde(default)]
        seed: Option<u64>,
        #[serde(default)]
        solution: bool,
    },
    Maze {
        #[serde(default)]
        width: Option<u8>,
        #[serde(default)]
        height: Option<u8>,
        #[serde(default)]
        seed: Option<u64>,
    },
    Holidays {
        #[serde(default)]
        zone: holidays::Zone,
        #[serde(default = "one")]
        count: u8,
    },
    Moon {},
    Countdown {
        label: String,
        date: NaiveDate,
    },
    Quote {},
    /// Horoscope hors ligne et gratuit, calculé sur le ciel réel par Barnum.
    Barnum {
        /// Signe (en français, accents facultatifs)…
        #[serde(default)]
        sign: Option<String>,
        /// … ou date de naissance, dont Barnum déduit le signe.
        #[serde(default)]
        birth_date: Option<NaiveDate>,
        /// Affiche la position des astres.
        #[serde(default)]
        sky: bool,
        /// Autre série de formules pour le même ciel (`--sel` de Barnum).
        #[serde(default)]
        variant: Option<String>,
    },
    /// Pictogramme dessiné : cœur, étoile, soleil, fleur, sourire.
    Picto {
        shape: picto::Shape,
        #[serde(default)]
        size: picto::Size,
        #[serde(default = "one")]
        count: u8,
    },
    /// Énigme du jour (devinette, charade, logique, calcul).
    #[serde(alias = "enigme")]
    Riddle {
        /// Limite à une famille : devinette, charade, logique, calcul.
        #[serde(default)]
        kind: Option<riddle::Kind>,
        /// Numéro d'une énigme précise (imprimé sous chaque énigme).
        #[serde(default)]
        number: Option<usize>,
        /// Réponse : envers (défaut), lendemain, dessous ou aucune.
        #[serde(default)]
        answer: riddle::Answer,
    },
    /// Petit bac : une lettre, des catégories, une feuille par joueur.
    PetitBac {
        #[serde(default)]
        letter: Option<char>,
        /// Catégories imposées ; tirées au hasard si vide.
        #[serde(default)]
        categories: Vec<String>,
        /// Nombre de catégories tirées au hasard.
        #[serde(default = "six")]
        count: u8,
        #[serde(default = "one")]
        players: u8,
        #[serde(default)]
        seed: Option<u64>,
    },
    /// Fiche de calcul mental, résultats à l'envers.
    #[serde(alias = "calcul_mental")]
    MentalMath {
        #[serde(default)]
        difficulty: sudoku::Difficulty,
        #[serde(default = "ten")]
        count: u8,
        #[serde(default)]
        seed: Option<u64>,
    },
    /// Mots mystères : lettres mélangées, réponses à l'envers.
    #[serde(alias = "mot_mystere", alias = "anagramme")]
    Anagram {
        #[serde(default)]
        theme: Option<word_search::Theme>,
        #[serde(default = "three")]
        count: u8,
        #[serde(default)]
        seed: Option<u64>,
    },
    /// Logimage (picross) : un dessin à révéler.
    #[serde(alias = "logimage", alias = "picross")]
    Nonogram {
        #[serde(default)]
        number: Option<usize>,
        #[serde(default)]
        solution: bool,
    },
    /// Message codé (César, morse, nombres).
    #[serde(alias = "message_code")]
    Cipher {
        #[serde(default)]
        message: Option<String>,
        #[serde(default)]
        cipher: cipher::Cipher,
        /// Décalage du code César (au hasard si absent).
        #[serde(default)]
        shift: Option<u8>,
        /// Message en clair, imprimé à l'envers en bas.
        #[serde(default = "yes")]
        answer: bool,
        #[serde(default)]
        seed: Option<u64>,
    },
    /// QR code de connexion au Wi-Fi.
    Wifi {
        ssid: String,
        #[serde(default)]
        password: Option<String>,
        #[serde(default)]
        security: wifi::Security,
        #[serde(default)]
        hidden: bool,
        #[serde(default = "yes")]
        show_password: bool,
    },
    /// Rappel des poubelles à sortir.
    #[serde(alias = "poubelles")]
    Bins {
        collections: Vec<bins::Collection>,
        #[serde(default)]
        when: bins::When,
        /// Imprime aussi les soirs sans ramassage (avec le prochain).
        #[serde(default)]
        always: bool,
    },
    /// Mandala à colorier.
    #[serde(alias = "coloriage")]
    Coloring {
        #[serde(default)]
        seed: Option<u64>,
    },
}

/// Ressources partagées par les blocs.
pub struct Ctx {
    pub today: NaiveDate,
    /// Aperçu : pas de glitch glissé au hasard dans le ticket.
    pub preview: bool,
}

impl Ctx {
    pub fn new() -> Self {
        Self { today: chrono::Local::now().date_naive(), preview: false }
    }
}

/// Bilan de la construction d'un bloc, pour la console et les réponses HTTP.
pub struct Report {
    pub label: String,
    pub elapsed: Duration,
    pub cached: bool,
    pub error: Option<String>,
}

impl Block {
    pub fn name(&self) -> &'static str {
        match self {
            Block::PetitBac { .. } => "petit bac",
            Block::MentalMath { .. } => "calcul mental",
            Block::Anagram { .. } => "mot mystère",
            Block::Nonogram { .. } => "logimage",
            Block::Cipher { .. } => "message codé",
            Block::Wifi { .. } => "Wi-Fi",
            Block::Bins { .. } => "poubelles",
            Block::Coloring { .. } => "coloriage",
            Block::Title { .. } => "titre",
            Block::Text { .. } => "texte",
            Block::Glitch { .. } => "glitch",
            Block::Separator { .. } => "séparateur",
            Block::Date {} => "date",
            Block::Feed { .. } => "espace",
            Block::Image { .. } => "image",
            Block::Qr { .. } => "QR code",
            Block::Saint {} => "saint du jour",
            Block::Todo { .. } => "à faire",
            Block::Sudoku { .. } => "sudoku",
            Block::WordSearch { .. } => "mots mêlés",
            Block::TrainTracks { .. } => "voie ferrée",
            Block::Maze { .. } => "labyrinthe",
            Block::Holidays { .. } => "jours fériés",
            Block::Moon {} => "lune",
            Block::Countdown { .. } => "compte à rebours",
            Block::Quote {} => "citation",
            Block::Riddle { .. } => "énigme",
            Block::Picto { .. } => "pictogramme",
            Block::Barnum { .. } => "horoscope Barnum",
        }
    }

    /// Nom du bloc, précisé par son paramètre principal : « météo · Lyon ».
    pub fn label(&self) -> String {
        let detail = match self {
            Block::Title { text, .. } => Some(text.chars().take(24).collect()),
            Block::Sudoku { difficulty, solution, .. } | Block::TrainTracks { difficulty, solution, .. } => {
                Some(format!("{}{}", difficulty.label(), if *solution { ", solution" } else { "" }))
            }
            Block::WordSearch { difficulty, theme, words, solution, .. } => {
                let subject = match (words.is_empty(), theme) {
                    (false, _) => "mes mots, ",
                    (true, Some(t)) => &format!("{}, ", t.label()),
                    (true, None) => "",
                };
                Some(format!("{subject}{}{}", difficulty.label(), if *solution { ", solution" } else { "" }))
            }
            Block::MentalMath { difficulty, .. } => Some(difficulty.label().to_owned()),
            Block::Cipher { cipher, .. } => Some(cipher.label().to_owned()),
            Block::Wifi { ssid, .. } => Some(ssid.clone()),
            Block::Nonogram { number, .. } => number.map(|n| format!("n° {n}")),
            Block::Countdown { label, .. } => Some(label.clone()),
            Block::Picto { shape, .. } => Some(shape.label().to_owned()),
            Block::Barnum { sign, birth_date, .. } => {
                birth_date.map(|d| format!("né le {d}")).or_else(|| sign.clone())
            }
            Block::Image { path, .. } => Some(path.rsplit('/').next().unwrap_or(path).to_owned()),
            _ => None,
        };
        match detail {
            Some(d) => format!("{} · {d}", self.name()),
            None => self.name().to_owned(),
        }
    }

    pub fn build(&self, ctx: &Ctx) -> Result<Doc> {
        let mut doc = Doc::new();
        match self {
            Block::Title { text, size } => {
                doc.text(text, Style::default().bold().center().size(*size));
            }
            Block::Text { text, bold, underline, reverse, small, size, align } => {
                let style = Style { bold: *bold, underline: *underline, reverse: *reverse, small: *small, align: *align, ..Style::default() };
                doc.text(text, style.size(*size));
            }
            Block::Glitch { seed } => doc = glitch::build(*seed),
            Block::Separator { style } => {
                doc.rule(style.unwrap_or('-'));
            }
            Block::Date {} => {
                doc.text(&fr::capitalize(&fr::long_date(ctx.today)), Style::default().bold().center());
            }
            Block::Feed { lines } => {
                doc.feed(*lines);
            }
            Block::Image { path, dither } => {
                doc.image(raster::load(path.as_ref(), *dither)?);
            }
            Block::Qr { data, size, caption } => {
                doc.qr(data, size.unwrap_or(6));
                if let Some(caption) = caption {
                    doc.text(caption, Style::default().small().center());
                }
            }
            Block::Saint {} => doc = saint::build(ctx.today),
            Block::Todo { title, items } => {
                doc.header(title.as_deref().unwrap_or("À faire"));
                for item in items {
                    doc.hanging("[ ] ", item, Style::default());
                }
            }
            Block::Sudoku { difficulty, seed, solution } => doc = sudoku::build(*difficulty, *seed, *solution),
            Block::WordSearch { difficulty, theme, words, seed, solution } => {
                doc = word_search::build(*difficulty, *theme, words, *seed, *solution);
            }
            Block::TrainTracks { difficulty, seed, solution } => {
                doc = train_tracks::build(*difficulty, *seed, *solution);
            }
            Block::Maze { width, height, seed } => {
                doc = maze::build(width.unwrap_or(12), height.unwrap_or(16), *seed);
            }
            Block::Holidays { zone, count } => doc = holidays::build(ctx.today, *zone, *count),
            Block::Moon {} => doc = moon::build(chrono::Utc::now()),
            Block::Countdown { label, date } => {
                let days = (*date - ctx.today).num_days();
                let (big, small) = match days {
                    0 => ("C'est aujourd'hui !".to_owned(), label.clone()),
                    d if d > 0 => (format!("J-{d}"), format!("avant : {label}")),
                    d => (format!("J+{}", -d), format!("depuis : {label}")),
                };
                let size = if big.chars().count() <= 14 { 3 } else { 1 };
                doc.text(&big, Style::default().bold().center().size(size));
                doc.text(&small, Style::default().center());
            }
            Block::Quote {} => doc = quote::build(ctx.today),
            Block::Riddle { kind, number, answer } => doc = riddle::build(ctx.today, *kind, *number, *answer)?,
            Block::Picto { shape, size, count } => doc = picto::build(*shape, *size, *count),
            Block::Barnum { sign, birth_date, sky, variant } => {
                let who = match (birth_date, sign) {
                    (Some(birth), _) => barnum::Who::Birth(*birth),
                    (None, Some(sign)) => barnum::Who::Sign(sign.clone()),
                    (None, None) => anyhow::bail!("indiquer `sign` ou `birth_date`"),
                };
                doc = barnum::build(who, ctx.today, *sky, variant.as_deref())?;
            }
            Block::PetitBac { letter, categories, count, players, seed } => {
                doc = petit_bac::build(*letter, categories, *count, *players, *seed);
            }
            Block::MentalMath { difficulty, count, seed } => doc = mental_math::build(*difficulty, *count, *seed),
            Block::Anagram { theme, count, seed } => doc = anagram::build(*theme, *count, *seed),
            Block::Nonogram { number, solution } => doc = nonogram::build(*number, *solution)?,
            Block::Cipher { message, cipher: kind, shift, answer, seed } => {
                doc = cipher::build(message.as_deref(), *kind, *shift, *answer, *seed);
            }
            Block::Wifi { ssid, password, security, hidden, show_password } => {
                doc = wifi::build(ssid, password.as_deref(), *security, *hidden, *show_password)?;
            }
            Block::Bins { collections, when, always } => doc = bins::build(collections, *when, *always, ctx.today)?,
            Block::Coloring { seed } => doc = coloring::build(*seed),
        }
        Ok(doc)
    }
}

impl Ticket {
    /// Construit tous les blocs en parallèle (les requêtes réseau partent en même temps),
    /// puis les assemble dans l'ordre. Un bloc en échec est remplacé par un message.
    /// `progress` est prévenu au fil de l'eau, pour l'affichage en console.
    pub fn build(&self, ctx: &Ctx, progress: &mut dyn Progress) -> (Doc, Vec<Report>) {
        let labels: Vec<String> = self.blocks.iter().map(Block::label).collect();
        let mut done: Vec<Option<(Doc, Report)>> = self.blocks.iter().map(|_| None).collect();
        progress.start(&labels);

        std::thread::scope(|s| {
            let (tx, rx) = mpsc::channel();
            for (i, block) in self.blocks.iter().enumerate() {
                let tx = tx.clone();
                s.spawn(move || {
                    let start = Instant::now();
                    let result = std::panic::catch_unwind(AssertUnwindSafe(|| block.build(ctx)))
                        .unwrap_or_else(|_| Err(anyhow::anyhow!("erreur interne")))
                        .map_err(|e| format!("{e:#}"));
                    let _ = tx.send((i, result, start.elapsed()));
                });
            }
            drop(tx);

            loop {
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok((i, result, elapsed)) => {
                        let block = &self.blocks[i];
                        let (doc, error) = match result {
                            Ok(doc) => (doc, None),
                            Err(error) => {
                                let mut doc = Doc::new();
                                let message = format!("({} indisponible : {error})", block.name());
                                doc.text(&message, Style::default().small().center());
                                (doc, Some(error))
                            }
                        };
                        let report = Report { label: labels[i].clone(), elapsed, cached: doc.from_cache, error };
                        progress.done(i, &report);
                        done[i] = Some((doc, report));
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => progress.tick(),
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });
        progress.finish();

        // De temps en temps, la machine glisse un message entre deux blocs, sans rien dire en console.
        let forced = self.blocks.iter().any(|b| matches!(b, Block::Glitch { .. }));
        let glitch_at = (!ctx.preview && !forced && glitch::chance()).then(|| rand::rng().random_range(0..=done.len()));

        let mut ticket = Doc::new();
        let mut reports = Vec::with_capacity(done.len());
        let mut parts: Vec<Doc> = Vec::with_capacity(done.len() + 1);
        for (doc, report) in done.into_iter().map(|d| d.expect("chaque bloc a répondu")) {
            parts.push(doc);
            reports.push(report);
        }
        if let Some(at) = glitch_at {
            parts.insert(at, glitch::build(None));
        }
        for (i, part) in parts.into_iter().enumerate() {
            if i > 0 {
                ticket.feed(self.spacing);
            }
            ticket.append(part);
        }
        (ticket, reports)
    }
}

/// Suivi de la construction d'un ticket (affichage en console).
pub trait Progress {
    fn start(&mut self, _labels: &[String]) {}
    /// Appelé régulièrement tant que des blocs sont en cours.
    fn tick(&mut self) {}
    fn done(&mut self, _index: usize, _report: &Report) {}
    fn finish(&mut self) {}
}

/// Aucun affichage (serveur, mode silencieux).
pub struct Silent;

impl Progress for Silent {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ticket_with_defaults() {
        let t: Ticket = serde_json::from_str(
            r#"{"blocks": [
                {"type": "title", "text": "Bonjour"},
                {"type": "saint"},
                {"type": "voie_ferree"},
                {"type": "sudoku", "difficulty": "facile"},
                {"type": "mots_meles"}
            ]}"#,
        )
        .unwrap();
        assert!(t.cut);
        assert_eq!(t.blocks.len(), 5);
    }

    #[test]
    fn parses_new_daily_blocks_and_defaults() {
        let t: Ticket = serde_json::from_str(
            r#"{"blocks": [
                {"type": "calcul_mental"},
                {"type": "enigme", "kind": "charade", "answer": "lendemain"}
            ]}"#,
        )
        .unwrap();
        assert!(matches!(t.blocks[0], Block::MentalMath { .. }));
        assert!(matches!(t.blocks[1], Block::Riddle { kind: Some(riddle::Kind::Charade), .. }));
    }

    /// Chaque ticket que la borne peut produire (chaque jeu, chaque choix de chaque réglage,
    /// chaque pack, chaque solution) est un ticket valide, dont tous les blocs s'impriment sans erreur.
    #[test]
    fn kiosk_tickets_are_valid() {
        let config = borne_ui::Config::load(std::path::Path::new("kiosk")).unwrap();
        let ctx = Ctx { today: NaiveDate::from_ymd_opt(2026, 10, 7).unwrap(), preview: true };
        let check = |json: serde_json::Value, title: &str| {
            let text = json.to_string();
            let ticket: Ticket = serde_json::from_value(json).unwrap_or_else(|e| panic!("« {title} » : {e}\n{text}"));
            let (_, reports) = ticket.build(&ctx, &mut Silent);
            let errors: Vec<_> = reports.iter().filter_map(|r| r.error.as_ref()).collect();
            assert!(errors.is_empty(), "« {title} » : {errors:?}\n{text}");
        };
        for (g, game) in config.games.iter().enumerate() {
            if game.is_pack() {
                // Le pack avec tous les jeux qu'il peut tirer.
                check(config.pack_ticket(g, &config.pack_pool(g)), &game.title);
                continue;
            }
            let default = config.default_selection(g);
            let mut selections = vec![default.clone()];
            for (o, option) in game.options.iter().enumerate() {
                for c in 0..option.choices.len() {
                    let mut selection = default.clone();
                    selection[o] = c;
                    selections.push(selection);
                }
            }
            for selection in &selections {
                check(config.ticket(g, selection), &game.title);
            }
            // Solution : le plus petit et le plus grand numéro acceptés, pour chaque réglage.
            if let Some(solution) = &game.solution {
                for number in [1, solution.max.unwrap_or(99_999)] {
                    for selection in &selections {
                        check(config.solution_ticket(g, selection, number), &game.title);
                    }
                }
            }
        }
    }

    #[test]
    fn rejects_unknown_fields_and_types() {
        assert!(serde_json::from_str::<Ticket>(r#"{"blocks": [{"type": "saint", "oups": 1}]}"#).is_err());
        assert!(serde_json::from_str::<Ticket>(r#"{"blocks": [{"type": "licorne"}]}"#).is_err());
    }

    #[test]
    fn failing_block_does_not_break_ticket() {
        let t: Ticket = serde_json::from_str(
            r#"{"blocks": [{"type": "text", "text": "avant"}, {"type": "image", "path": "/nope.png"}, {"type": "text", "text": "après"}]}"#,
        )
        .unwrap();
        let ctx = Ctx { today: NaiveDate::from_ymd_opt(2026, 10, 7).unwrap(), preview: false };
        let (doc, reports) = t.build(&ctx, &mut Silent);
        let preview = doc.preview(true);
        assert_eq!(reports.iter().filter(|r| r.error.is_some()).count(), 1);
        assert!(preview.contains("avant") && preview.contains("après"));
        assert!(preview.contains("image indisponible"));
    }

    #[test]
    fn build_reports_each_block_and_keeps_ticket_order() {
        let t: Ticket = serde_json::from_str(
            r#"{"blocks": [
                {"type": "title", "text": "premier"},
                {"type": "image", "path": "/nope.png"},
                {"type": "title", "text": "dernier"}
            ]}"#,
        )
        .unwrap();
        let ctx = Ctx { today: NaiveDate::from_ymd_opt(2026, 10, 7).unwrap(), preview: false };
        /// Note l'ordre dans lequel les blocs se terminent.
        struct Recorder(Vec<String>);
        impl Progress for Recorder {
            fn done(&mut self, _index: usize, report: &Report) {
                self.0.push(report.label.clone());
            }
        }
        let mut completed = Recorder(Vec::new());
        let (doc, reports) = t.build(&ctx, &mut completed);

        assert_eq!(completed.0.len(), 3);
        assert_eq!(
            reports
                .iter()
                .map(|report| report.label.as_str())
                .collect::<Vec<_>>(),
            ["titre · premier", "image · nope.png", "titre · dernier"]
        );
        let titles: Vec<_> = doc
            .ops
            .iter()
            .filter_map(|op| match op {
                crate::doc::Op::Line { text, .. } if text == "premier" || text == "dernier" => {
                    Some(text.as_str())
                }
                _ => None,
            })
            .collect();
        assert_eq!(titles, ["premier", "dernier"]);
    }
}
