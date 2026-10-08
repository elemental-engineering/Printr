//! Représentation intermédiaire d'un ticket : lignes stylées, images, QR codes.
//! Les blocs produisent un `Doc`, qu'on imprime ou qu'on prévisualise en texte.

use anyhow::Result;
use escpos::driver::Driver;
use escpos::printer::Printer;
use escpos::utils::*;
use image::GrayImage;
use serde::Deserialize;

use crate::{cp858, raster};

/// Colonnes en police A (12 points) et B (9 points) sur 512 points.
const COLUMNS_A: usize = 42;
const COLUMNS_B: usize = 56;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub underline: bool,
    pub reverse: bool,
    /// Police B, plus petite.
    pub small: bool,
    /// Agrandissement en largeur et hauteur, de 1 à 8.
    pub size: u8,
    pub align: Align,
    /// Texte imprimé à l'envers (tourné à 180°) : on retourne le ticket pour le lire.
    pub upside_down: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            bold: false,
            underline: false,
            reverse: false,
            small: false,
            size: 1,
            align: Align::Left,
            upside_down: false,
        }
    }
}

impl Style {
    pub fn bold(self) -> Self {
        Self { bold: true, ..self }
    }
    pub fn underline(self) -> Self {
        Self { underline: true, ..self }
    }
    pub fn reverse(self) -> Self {
        Self { reverse: true, ..self }
    }
    pub fn small(self) -> Self {
        Self { small: true, ..self }
    }
    pub fn size(self, size: u8) -> Self {
        Self { size: size.clamp(1, 8), ..self }
    }
    pub fn align(self, align: Align) -> Self {
        Self { align, ..self }
    }
    pub fn upside_down(self) -> Self {
        Self { upside_down: true, ..self }
    }
    pub fn center(self) -> Self {
        self.align(Align::Center)
    }

    /// Nombre de caractères par ligne dans ce style.
    pub fn columns(&self) -> usize {
        let base = if self.small { COLUMNS_B } else { COLUMNS_A };
        (base / self.size as usize).max(1)
    }
}

pub enum Op {
    Line { text: String, style: Style },
    Image(GrayImage),
    Qr { data: String, size: u8 },
    Feed(u8),
}

#[derive(Default)]
pub struct Doc {
    pub ops: Vec<Op>,
    /// Contenu repris du cache plutôt que généré (pour le rapport en console).
    pub from_cache: bool,
}

impl Doc {
    pub fn new() -> Self {
        Self::default()
    }

    /// Texte libre : coupé aux mots selon la largeur du style ; une ligne vide saute une ligne.
    pub fn text(&mut self, text: &str, style: Style) -> &mut Self {
        self.hanging("", text, style)
    }

    /// Texte dont la première ligne commence par `prefix` et les suivantes sont
    /// indentées d'autant (puces, cases à cocher, étiquettes).
    pub fn hanging(&mut self, prefix: &str, text: &str, style: Style) -> &mut Self {
        let prefix = cp858::normalize(prefix);
        let indent = " ".repeat(prefix.chars().count());
        let width = style.columns().saturating_sub(indent.len()).max(1);
        let mut ops = Vec::new();
        for paragraph in cp858::normalize(text).lines() {
            if paragraph.trim().is_empty() {
                ops.push(Op::Feed(1));
                continue;
            }
            for (i, line) in wrap(paragraph, width).into_iter().enumerate() {
                let lead = if i == 0 { &prefix } else { &indent };
                ops.push(Op::Line { text: format!("{lead}{line}"), style });
            }
        }
        // À l'envers, chaque ligne est retournée : on les imprime aussi dans l'ordre inverse
        // pour que le texte se lise de haut en bas une fois le ticket retourné.
        if style.upside_down {
            ops.reverse();
        }
        self.ops.extend(ops);
        self
    }

    /// Une ligne telle quelle (espaces conservés), tronquée à la largeur du style.
    pub fn line(&mut self, text: &str, style: Style) -> &mut Self {
        let text = cp858::normalize(text).chars().take(style.columns()).collect();
        self.ops.push(Op::Line { text, style });
        self
    }

    /// Bandeau de section : texte inversé sur toute la largeur.
    pub fn header(&mut self, title: &str) -> &mut Self {
        let style = Style::default().bold().reverse();
        let title = cp858::normalize(title).to_uppercase();
        let width = style.columns();
        let title: String = title.chars().take(width).collect();
        let pad = width - title.chars().count();
        let line = format!("{}{}{}", " ".repeat(pad / 2), title, " ".repeat(pad - pad / 2));
        self.ops.push(Op::Line { text: line, style });
        self
    }

    /// Ligne horizontale faite du caractère `ch`.
    pub fn rule(&mut self, ch: char) -> &mut Self {
        let style = Style::default();
        self.ops.push(Op::Line { text: ch.to_string().repeat(style.columns()), style });
        self
    }

    pub fn feed(&mut self, lines: u8) -> &mut Self {
        if lines > 0 {
            self.ops.push(Op::Feed(lines));
        }
        self
    }

    pub fn image(&mut self, img: GrayImage) -> &mut Self {
        self.ops.push(Op::Image(img));
        self
    }

    pub fn qr(&mut self, data: &str, size: u8) -> &mut Self {
        self.ops.push(Op::Qr { data: data.to_owned(), size: size.clamp(1, 16) });
        self
    }

    pub fn append(&mut self, other: Doc) -> &mut Self {
        self.ops.extend(other.ops);
        self
    }

    /// Envoie le document à l'imprimante (sans l'imprimer : appeler `print`/`print_cut`).
    pub fn render<D: Driver>(&self, printer: &mut Printer<D>) -> Result<()> {
        let mut current: Option<Style> = None;
        let mut text_area = false;
        for op in &self.ops {
            // Les lignes de texte occupent 504 points (42 × 12, 56 × 9) : zone d'impression
            // centrée pour elles, pleine largeur pour les images et QR codes.
            let wants_text_area = matches!(op, Op::Line { .. });
            if wants_text_area != text_area {
                set_print_area(printer, wants_text_area)?;
                text_area = wants_text_area;
            }
            match op {
                Op::Line { text, style } => {
                    if current != Some(*style) {
                        apply_style(printer, style)?;
                        current = Some(*style);
                    }
                    let mut bytes = cp858::encode(text);
                    bytes.push(b'\n');
                    printer.custom(&bytes)?;
                }
                Op::Image(img) => {
                    raster::print_gray(printer, img)?;
                }
                Op::Qr { data, size } => {
                    printer.justify(JustifyMode::CENTER)?.qrcode_option(
                        data,
                        QRCodeOption::new(QRCodeModel::Model2, *size, QRCodeCorrectionLevel::M),
                    )?;
                    current = None;
                }
                Op::Feed(n) => {
                    printer.feeds(*n)?;
                }
            }
        }
        if text_area {
            set_print_area(printer, false)?;
        }
        apply_style(printer, &Style::default())?;
        Ok(())
    }

    /// Hauteur estimée en points (180 points par pouce), pour annoncer la longueur de papier.
    pub fn height_dots(&self) -> u32 {
        self.ops
            .iter()
            .map(|op| match op {
                Op::Line { style, .. } => (24 * style.size as u32 + 6).max(30),
                Op::Image(img) => img.height(),
                Op::Qr { data, size } => qr_modules(data.len()) * *size as u32 + 30,
                Op::Feed(n) => 30 * *n as u32,
            })
            .sum()
    }

    /// Aperçu du ticket encadré comme un ruban de papier, avec styles ANSI
    /// (à afficher via `anstream`, qui les retire hors terminal).
    pub fn preview_styled(&self, cut: bool) -> String {
        let frame = anstyle::Style::new().dimmed();
        let mut out = String::new();
        out.push_str(&format!("{frame}╭{}╮{frame:#}\n", "─".repeat(PAPER + 2)));
        let mut push = |content: String, visible: usize| {
            let fill = " ".repeat(PAPER.saturating_sub(visible));
            out.push_str(&format!("{frame}│{frame:#} {content}{fill} {frame}│{frame:#}\n"));
        };
        for op in &self.ops {
            match op {
                Op::Line { text, style } => {
                    let (content, visible) = styled_line(text, style);
                    push(content, visible);
                }
                Op::Image(img) => {
                    for row in image_preview(img) {
                        push(row, PAPER);
                    }
                }
                Op::Qr { data, .. } => {
                    let label = format!("▣ QR : {data}");
                    let label: String = label.chars().take(PAPER).collect();
                    let len = label.chars().count();
                    let pad = (PAPER - len) / 2;
                    let dim = anstyle::Style::new().dimmed();
                    push(format!("{}{dim}{label}{dim:#}", " ".repeat(pad)), pad + len);
                }
                Op::Feed(n) => (0..*n).for_each(|_| push(String::new(), 0)),
            }
        }
        if cut {
            let side = (PAPER - 1) / 2;
            out.push_str(&format!(
                "{frame}╰{} ✂ {}╯{frame:#}\n",
                "╌".repeat(side),
                "╌".repeat(PAPER - 1 - side)
            ));
        } else {
            out.push_str(&format!("{frame}╰{}╯{frame:#}\n", "─".repeat(PAPER + 2)));
        }
        out
    }

    /// Aperçu sans couleurs (tests).
    #[cfg(test)]
    pub fn preview(&self, cut: bool) -> String {
        anstream::adapter::strip_str(&self.preview_styled(cut)).to_string()
    }
}

/// Largeur de l'aperçu : celle de la police B, la plus étroite.
const PAPER: usize = COLUMNS_B;

/// Nombre de modules d'un QR code (niveau M) selon la longueur des données.
fn qr_modules(len: usize) -> u32 {
    match len {
        0..=14 => 21,
        15..=26 => 25,
        27..=42 => 29,
        43..=62 => 33,
        63..=84 => 37,
        _ => 45,
    }
}

/// Ligne d'aperçu : la police A (42 colonnes) est centrée sur la largeur du papier (56),
/// les agrandissements sont simulés en espaçant les lettres.
fn styled_line(text: &str, style: &Style) -> (String, usize) {
    let shown: String = if style.size > 1 {
        let gap = " ".repeat(style.size as usize - 1);
        text.chars().map(|c| format!("{c}{gap}")).collect::<String>().trim_end().to_owned()
    } else {
        text.to_owned()
    };
    let width = if style.small { COLUMNS_B } else { COLUMNS_A };
    let margin = (PAPER - width) / 2;
    let len = shown.chars().count().min(width);
    let shown: String = shown.chars().take(len).collect();
    let pad = match style.align {
        Align::Left => 0,
        Align::Center => (width - len) / 2,
        Align::Right => width - len,
    };
    let mut ansi = anstyle::Style::new();
    if style.bold {
        ansi = ansi.bold();
    }
    if style.underline {
        ansi = ansi.underline();
    }
    if style.reverse {
        ansi = ansi.invert();
    }
    // Aperçu d'une ligne à l'envers : caractères en ordre inverse, en grisé.
    let shown = if style.upside_down {
        ansi = ansi.dimmed();
        shown.chars().rev().collect()
    } else {
        shown
    };
    (format!("{}{ansi}{shown}{ansi:#}", " ".repeat(margin + pad)), margin + pad + len)
}

/// Largeur occupée par une ligne de texte, en points (identique en police A et B).
const TEXT_WIDTH: u32 = COLUMNS_A as u32 * 12;

/// `GS L` + `GS W` : zone d'impression centrée de `TEXT_WIDTH` points, ou toute la largeur.
fn set_print_area<D: Driver>(printer: &mut Printer<D>, text: bool) -> Result<()> {
    let (left, width) = if text {
        ((raster::PRINT_WIDTH - TEXT_WIDTH) / 2, TEXT_WIDTH)
    } else {
        (0, raster::PRINT_WIDTH)
    };
    let [l0, l1, ..] = left.to_le_bytes();
    let [w0, w1, ..] = width.to_le_bytes();
    printer.custom(&[0x1d, b'L', l0, l1, 0x1d, b'W', w0, w1])?;
    Ok(())
}

fn apply_style<D: Driver>(printer: &mut Printer<D>, style: &Style) -> Result<()> {
    let justify = match style.align {
        Align::Left => JustifyMode::LEFT,
        Align::Center => JustifyMode::CENTER,
        Align::Right => JustifyMode::RIGHT,
    };
    printer
        .justify(justify)?
        .bold(style.bold)?
        .underline(if style.underline { UnderlineMode::Single } else { UnderlineMode::None })?
        .reverse(style.reverse)?
        .font(if style.small { Font::B } else { Font::A })?
        .size(style.size, style.size)?
        .upside_down(style.upside_down)?;
    Ok(())
}

/// Coupe un paragraphe aux espaces ; les mots trop longs sont coupés net.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut len = 0;
    for word in text.split_whitespace() {
        let mut word: Vec<char> = word.chars().collect();
        while word.len() > width {
            if len > 0 {
                lines.push(std::mem::take(&mut line));
                len = 0;
            }
            lines.push(word.drain(..width).collect());
        }
        if word.is_empty() {
            continue;
        }
        if len > 0 && len + 1 + word.len() > width {
            lines.push(std::mem::take(&mut line));
            len = 0;
        }
        if len > 0 {
            line.push(' ');
            len += 1;
        }
        line.extend(word.iter());
        len += word.len();
    }
    if len > 0 {
        lines.push(line);
    }
    lines
}

/// Rendu d'une image en demi-blocs, sur la largeur de l'aperçu.
fn image_preview(img: &GrayImage) -> Vec<String> {
    let (w, h) = img.dimensions();
    let step = w as f64 / PAPER as f64; // points par colonne, et par demi-ligne
    let dark = |col: usize, half_row: usize| {
        let (x0, y0) = ((col as f64 * step) as u32, (half_row as f64 * step) as u32);
        let (x1, y1) = (((col + 1) as f64 * step) as u32, ((half_row + 1) as f64 * step) as u32);
        let mut black = 0;
        let mut total = 0;
        for y in y0..y1.min(h) {
            for x in x0..x1.min(w) {
                total += 1;
                black += u32::from(img.get_pixel(x, y).0[0] < 128);
            }
        }
        total > 0 && black * 4 > total
    };
    let rows = (h as f64 / (2.0 * step)).ceil() as usize;
    (0..rows)
        .map(|r| {
            (0..PAPER)
                .map(|c| match (dark(c, 2 * r), dark(c, 2 * r + 1)) {
                    (true, true) => '█',
                    (true, false) => '▀',
                    (false, true) => '▄',
                    (false, false) => ' ',
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_on_words() {
        assert_eq!(wrap("le petit chat est mort", 10), vec!["le petit", "chat est", "mort"]);
        assert_eq!(wrap("anticonstitutionnellement", 10), vec!["anticonsti", "tutionnell", "ement"]);
        assert!(wrap("   ", 10).is_empty());
    }

    #[test]
    fn hanging_indent() {
        let mut doc = Doc::new();
        doc.hanging("[ ] ", &"mot ".repeat(15), Style::default());
        let texts: Vec<&str> = doc
            .ops
            .iter()
            .filter_map(|op| match op {
                Op::Line { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts[0].starts_with("[ ] mot"));
        assert!(texts[1].starts_with("    mot"));
        assert!(texts.iter().all(|l| l.chars().count() <= 42));
        let preview = doc.preview(true);
        assert!(preview.lines().all(|l| l.chars().count() == PAPER + 4));
    }

    #[test]
    fn columns_by_style() {
        assert_eq!(Style::default().columns(), 42);
        assert_eq!(Style::default().small().columns(), 56);
        assert_eq!(Style::default().size(2).columns(), 21);
    }
}
