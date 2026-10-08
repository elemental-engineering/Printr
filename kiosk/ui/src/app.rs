//! Écrans de la borne et leur logique : accueil (grille de jeux), réglages d'un jeu, impression.
//! Tout est dessiné avec `embedded-graphics` sur un écran de 480 × 272 ; le matériel (écran,
//! tactile, imprimante) reste à l'appelant, qui transmet les appuis et le temps qui passe.

use std::collections::BTreeMap;

use embedded_graphics::mono_font::iso_8859_1::{
    FONT_7X13, FONT_8X13, FONT_8X13_BOLD, FONT_9X15_BOLD, FONT_9X18_BOLD, FONT_10X20,
};
use embedded_graphics::mono_font::{MonoFont, MonoTextStyle};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle, Triangle};
use embedded_graphics::text::{Alignment, Baseline, Text, TextStyleBuilder};

use crate::config::{Config, GameOption, Icon, Pick};
use crate::status::{format_offset, Clock, NtpStatus, WifiStatus, WIFI_ICON};

pub const WIDTH: u32 = 480;
pub const HEIGHT: u32 = 272;

const BAR: u32 = 36;
const COLUMNS: usize = 4;
const ROWS: usize = 2;
const PER_PAGE: usize = COLUMNS * ROWS;
/// Sans appui pendant ce temps, la borne revient à l'accueil.
const IDLE_MS: u32 = 60_000;
const DONE_MS: u32 = 5_000;
const FAILED_MS: u32 = 10_000;
/// Appui sur le titre de l'accueil qui ouvre le mode maintenance.
const HOLD_MS: u32 = 6_000;
/// Au-delà, une barre de progression montre que l'appui long est pris en compte.
const HOLD_HINT_MS: u32 = 1_000;
/// Durée d'une image de l'animation « recherche du réseau » de l'icône Wi-Fi.
const SCAN_FRAME_MS: u32 = 400;
/// Une impression sur `GLITCH_ODDS` est précédée d'un ticket glitch (0,001 %).
const GLITCH_ODDS: u64 = 100_000;

const fn rgb(r: u8, g: u8, b: u8) -> Rgb565 {
    Rgb565::new(r >> 3, g >> 2, b >> 3)
}

const BACKGROUND: Rgb565 = rgb(0xF6, 0xF0, 0xE6);
const BAR_COLOR: Rgb565 = rgb(0x4A, 0x2C, 0x2A);
const ACCENT: Rgb565 = rgb(0xE2, 0x60, 0x0E);
const CARD: Rgb565 = rgb(0xFF, 0xFF, 0xFF);
const BORDER: Rgb565 = rgb(0xD8, 0xCC, 0xBC);
const INK: Rgb565 = rgb(0x2B, 0x1D, 0x14);
const MUTED: Rgb565 = rgb(0x7A, 0x6A, 0x5A);
const WHITE: Rgb565 = rgb(0xFF, 0xFF, 0xFF);
/// Fond des tuiles de packs.
const PACK_CARD: Rgb565 = rgb(0xFD, 0xE7, 0xD6);

// Mode maintenance : thème sombre, bandeau noir souligné d'ambre, pour ne pas le confondre avec la
// borne.
const M_BACKGROUND: Rgb565 = rgb(0x0F, 0x17, 0x2A);
const M_PANEL: Rgb565 = rgb(0x1E, 0x29, 0x3B);
const M_LINE: Rgb565 = rgb(0x33, 0x41, 0x55);
const M_BAR: Rgb565 = rgb(0x02, 0x06, 0x17);
const M_BAR_LINE: Rgb565 = rgb(0xF5, 0x9E, 0x0B);
const M_ICON_DIM: Rgb565 = rgb(0x47, 0x55, 0x69);
const M_TEXT: Rgb565 = rgb(0xE2, 0xE8, 0xF0);
const M_MUTED: Rgb565 = rgb(0x94, 0xA3, 0xB8);
const M_CYAN: Rgb565 = rgb(0x22, 0xD3, 0xEE);
const M_GREEN: Rgb565 = rgb(0x4A, 0xDE, 0x80);
const M_AMBER: Rgb565 = rgb(0xFB, 0xBF, 0x24);
const M_RED: Rgb565 = rgb(0xF8, 0x71, 0x71);

/// Ce que la borne reçoit de l'extérieur.
#[derive(Clone, Copy, Debug)]
pub enum Event {
    /// Appui bref sur l'écran, en points (équivaut à `Down` puis `Up` au même endroit).
    Tap(Point),
    /// Doigt posé sur l'écran.
    Down(Point),
    /// Doigt levé : un appui, sauf s'il a servi d'appui long.
    Up,
    /// Temps écoulé depuis le dernier `Tick`, en millisecondes.
    Tick(u32),
}

/// Ce que l'appelant doit faire après un événement.
#[derive(Debug, PartialEq)]
pub enum Effect {
    None,
    /// L'écran a changé : le redessiner.
    Redraw,
    /// Redessiner (écran « impression »), imprimer ces tickets JSON dans l'ordre (chacun coupé à
    /// la fin), puis appeler `print_finished`. Le dernier est celui demandé ; avant lui, parfois,
    /// un glitch sur son propre ticket.
    Print(Vec<String>),
}

#[derive(Debug, PartialEq)]
enum Screen {
    Home,
    Game,
    /// Saisie du numéro d'une grille, pour en imprimer la solution.
    Solution,
    Printing,
    Done,
    Failed(String),
    /// Mode maintenance (appui long sur le titre de l'accueil).
    Maintenance,
}

/// Doigt posé, pour détecter l'appui long.
#[derive(Clone, Copy, Debug)]
struct Press {
    at: Point,
    held: u32,
    /// Déjà utilisé comme appui long : le lever n'est pas un appui.
    used: bool,
}

pub struct App {
    config: Config,
    screen: Screen,
    page: usize,
    game: usize,
    selection: Vec<usize>,
    /// Temps sans appui, en millisecondes.
    idle: u32,
    /// Temps passé sur l'écran courant, en millisecondes.
    shown: u32,
    /// Nombre d'impressions de chaque jeu (par clé), pour les packs « populaires ».
    popularity: BTreeMap<String, u32>,
    /// État du générateur pseudo-aléatoire des packs « hasard ».
    rng: u64,
    /// Numéro de grille en cours de saisie.
    number: String,
    /// Ce qui s'imprime, affiché pendant l'impression.
    printing: String,
    /// Une impression sur `glitch_odds` commence par un glitch (0 : jamais).
    glitch_odds: u64,
    press: Option<Press>,
    wifi: WifiStatus,
    ntp: NtpStatus,
    clock: Option<Clock>,
    /// Temps écoulé, pour l'animation de l'icône Wi-Fi.
    anim: u32,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            screen: Screen::Home,
            page: 0,
            game: 0,
            selection: Vec::new(),
            idle: 0,
            shown: 0,
            popularity: BTreeMap::new(),
            rng: 0x2545_F491_4F6C_DD1D,
            number: String::new(),
            printing: String::new(),
            glitch_odds: GLITCH_ODDS,
            press: None,
            wifi: WifiStatus::Off,
            ntp: NtpStatus::Idle,
            clock: None,
            anim: 0,
        }
    }

    /// La carte doit-elle allumer le Wi-Fi et chercher le réseau de `config().maintenance` ?
    /// Vrai tant que le mode maintenance est ouvert : réessayer jusqu'à ce qu'il soit fermé.
    pub fn wifi_wanted(&self) -> bool {
        self.screen == Screen::Maintenance
    }

    #[cfg(test)]
    pub(crate) fn anim_ms(&self) -> u32 {
        self.anim
    }

    pub fn wifi(&self) -> &WifiStatus {
        &self.wifi
    }

    /// État du Wi-Fi, donné par la carte.
    pub fn set_wifi(&mut self, status: WifiStatus) -> Effect {
        if self.wifi == status {
            return Effect::None;
        }
        self.wifi = status;
        Effect::Redraw
    }

    /// La carte doit-elle synchroniser l'horloge par NTP ? Vrai en maintenance, Wi-Fi connecté.
    pub fn ntp_wanted(&self) -> bool {
        self.wifi_wanted() && matches!(self.wifi, WifiStatus::Connected { .. })
    }

    pub fn ntp(&self) -> NtpStatus {
        self.ntp
    }

    /// État de la synchronisation NTP, donné par la carte ; en `Synced`, à redonner chaque seconde
    /// avec l'écart du moment.
    pub fn set_ntp(&mut self, status: NtpStatus) -> Effect {
        if self.ntp == status {
            return Effect::None;
        }
        self.ntp = status;
        if self.ntp_wanted() { Effect::Redraw } else { Effect::None }
    }

    /// Heure de l'horloge de la carte, à donner chaque seconde.
    pub fn set_clock(&mut self, clock: Clock) -> Effect {
        let changed = self.clock != Some(clock);
        self.clock = Some(clock);
        if changed && self.screen == Screen::Maintenance { Effect::Redraw } else { Effect::None }
    }

    /// Graine du tirage au hasard (générateur matériel de l'ESP32, horloge…).
    pub fn seed(&mut self, seed: u64) {
        self.rng ^= seed;
        self.random();
    }

    /// Une impression sur combien commence par un glitch (0 : jamais).
    pub fn glitch_odds(&self) -> u64 {
        self.glitch_odds
    }

    /// Fréquence des glitchs en tête de ticket : un sur `odds` (0 pour jamais).
    pub fn set_glitch_odds(&mut self, odds: u64) {
        self.glitch_odds = odds;
    }

    /// Lance l'impression d'un ticket, précédé parfois d'un glitch, seul sur son ticket et coupé.
    fn print(&mut self, ticket: serde_json::Value) -> Effect {
        let mut tickets = Vec::with_capacity(2);
        if self.glitch_odds > 0 && self.random().is_multiple_of(self.glitch_odds) {
            tickets.push(serde_json::json!({ "cut": true, "blocks": [{ "type": "glitch" }] }).to_string());
        }
        tickets.push(ticket.to_string());
        self.go(Screen::Printing);
        Effect::Print(tickets)
    }

    /// Nombre d'impressions par jeu, à sauvegarder pour la prochaine mise sous tension.
    pub fn popularity(&self) -> &BTreeMap<String, u32> {
        &self.popularity
    }

    /// Reprend des statistiques sauvegardées.
    pub fn set_popularity(&mut self, popularity: BTreeMap<String, u32>) {
        self.popularity = popularity;
    }

    /// xorshift64* : largement suffisant pour tirer des jeux.
    fn random(&mut self) -> u64 {
        if self.rng == 0 {
            self.rng = 0x9E37_79B9_7F4A_7C15;
        }
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Jeux d'un pack, selon sa règle de choix.
    pub fn pack_games(&mut self, entry: usize) -> Vec<usize> {
        let mut pool = self.config.pack_pool(entry);
        let Some(pack) = &self.config.games[entry].pack else { return Vec::new() };
        let (pick, count) = (pack.pick, pack.count.min(pool.len()));
        match pick {
            Pick::Tous => pool,
            Pick::Hasard => {
                for i in 0..count {
                    let j = i + (self.random() % (pool.len() - i) as u64) as usize;
                    pool.swap(i, j);
                }
                pool.truncate(count);
                pool
            }
            Pick::Populaires => {
                // Tri stable : à égalité, l'ordre de games.json.
                let prints = |g: &usize| self.popularity.get(self.config.games[*g].key()).copied().unwrap_or(0);
                pool.sort_by_key(|g| std::cmp::Reverse(prints(g)));
                pool.truncate(count);
                pool
            }
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    fn pages(&self) -> usize {
        self.config.games.len().div_ceil(PER_PAGE)
    }

    fn go(&mut self, screen: Screen) -> Effect {
        self.screen = screen;
        self.shown = 0;
        Effect::Redraw
    }

    pub fn handle(&mut self, event: Event) -> Effect {
        match event {
            Event::Tap(p) => {
                self.idle = 0;
                self.tap(p)
            }
            Event::Down(p) => {
                self.idle = 0;
                self.press = Some(Press { at: p, held: 0, used: false });
                Effect::None
            }
            Event::Up => match self.press.take() {
                Some(press) if !press.used => {
                    self.idle = 0;
                    match self.tap(press.at) {
                        // La barre de progression de l'appui long doit disparaître.
                        Effect::None if press.held >= HOLD_HINT_MS => Effect::Redraw,
                        effect => effect,
                    }
                }
                _ => Effect::None,
            },
            Event::Tick(ms) => {
                self.idle = self.idle.saturating_add(ms);
                self.rng = self.rng.wrapping_add(u64::from(ms));
                self.shown = self.shown.saturating_add(ms);
                let frame = self.anim / SCAN_FRAME_MS;
                self.anim = self.anim.wrapping_add(ms);
                // L'icône Wi-Fi n'apparaît qu'en maintenance : jamais sur les écrans des joueurs.
                let animate = self.screen == Screen::Maintenance
                    && self.wifi == WifiStatus::Searching
                    && self.anim / SCAN_FRAME_MS != frame;
                let on_home = self.screen == Screen::Home;
                if let Some(press) = self.press.as_mut().filter(|p| !p.used && on_home && title_area().contains(p.at)) {
                    press.held = press.held.saturating_add(ms);
                    if press.held >= HOLD_MS {
                        press.used = true;
                        return self.go(Screen::Maintenance);
                    }
                    if press.held >= HOLD_HINT_MS {
                        return Effect::Redraw;
                    }
                }
                let effect = match self.screen {
                    Screen::Game | Screen::Solution if self.idle >= IDLE_MS => self.go(Screen::Home),
                    Screen::Home if self.idle >= IDLE_MS && self.page != 0 => {
                        self.page = 0;
                        Effect::Redraw
                    }
                    Screen::Done if self.shown >= DONE_MS => self.go(Screen::Home),
                    Screen::Failed(_) if self.shown >= FAILED_MS => self.go(Screen::Home),
                    _ => Effect::None,
                };
                if effect == Effect::None && animate { Effect::Redraw } else { effect }
            }
        }
    }

    /// Résultat de l'impression demandée par `Effect::Print`.
    pub fn print_finished(&mut self, result: Result<(), String>) -> Effect {
        if self.screen != Screen::Printing {
            return Effect::None;
        }
        match result {
            Ok(()) => self.go(Screen::Done),
            Err(message) => self.go(Screen::Failed(message)),
        }
    }

    fn tap(&mut self, p: Point) -> Effect {
        match self.screen {
            Screen::Home => {
                if self.pages() > 1 {
                    if page_prev().contains(p) && self.page > 0 {
                        self.page -= 1;
                        return Effect::Redraw;
                    }
                    if page_next().contains(p) && self.page + 1 < self.pages() {
                        self.page += 1;
                        return Effect::Redraw;
                    }
                }
                let slot = (0..PER_PAGE).find(|&slot| tile(slot).contains(p));
                match slot.map(|slot| self.page * PER_PAGE + slot) {
                    Some(game) if game < self.config.games.len() => {
                        self.game = game;
                        self.selection = self.config.default_selection(game);
                        self.go(Screen::Game)
                    }
                    _ => Effect::None,
                }
            }
            Screen::Game => {
                if back_button().contains(p) {
                    return self.go(Screen::Home);
                }
                if solution_button().contains(p) && self.config.games[self.game].solution.is_some() {
                    self.number.clear();
                    return self.go(Screen::Solution);
                }
                if print_button().contains(p) {
                    self.printing = self.config.games[self.game].title.clone();
                    let ticket = if self.config.games[self.game].is_pack() {
                        let chosen = self.pack_games(self.game);
                        self.config.pack_ticket(self.game, &chosen)
                    } else {
                        let key = self.config.games[self.game].key().to_owned();
                        *self.popularity.entry(key).or_default() += 1;
                        self.config.ticket(self.game, &self.selection)
                    };
                    return self.print(ticket);
                }
                let options = &self.config.games[self.game].options;
                for (i, option) in options.iter().enumerate() {
                    let n = option.choices.len();
                    let control = option_control(i, options.len());
                    let chosen = &mut self.selection[i];
                    match control_kind(n) {
                        Control::Segments => {
                            if let Some(k) = (0..n).find(|&k| segment(control, k, n).contains(p)) {
                                *chosen = k;
                                return Effect::Redraw;
                            }
                        }
                        Control::Selector => {
                            if selector_prev(control).contains(p) {
                                *chosen = (*chosen + n - 1) % n;
                                return Effect::Redraw;
                            }
                            if selector_next(control).contains(p) {
                                *chosen = (*chosen + 1) % n;
                                return Effect::Redraw;
                            }
                        }
                    }
                }
                Effect::None
            }
            Screen::Solution => self.tap_solution(p),
            Screen::Maintenance => {
                if maintenance_quit().contains(p) {
                    // À la prochaine maintenance, une nouvelle synchronisation.
                    self.ntp = NtpStatus::Idle;
                    self.go(Screen::Home)
                } else {
                    Effect::None
                }
            }
            Screen::Printing => Effect::None,
            Screen::Done | Screen::Failed(_) => self.go(Screen::Home),
        }
    }

    /// Numéro saisi, s'il est acceptable pour la solution du jeu courant.
    fn solution_number(&self) -> Option<u64> {
        let solution = self.config.games[self.game].solution.as_ref()?;
        self.number.parse().ok().filter(|&n| solution.accepts(n))
    }

    fn tap_solution(&mut self, p: Point) -> Effect {
        if back_button().contains(p) {
            return self.go(Screen::Game);
        }
        if keypad_print().contains(p) {
            let Some(number) = self.solution_number() else { return Effect::None };
            self.printing = format!("Solution {} n° {number}", self.config.games[self.game].title);
            let ticket = self.config.solution_ticket(self.game, &self.selection, number);
            return self.print(ticket);
        }
        let Some(key) = (0..KEYS.len()).find(|&k| keypad_key(k).contains(p)) else { return Effect::None };
        match KEYS[key] {
            "C" => self.number.clear(),
            "<" => {
                self.number.pop();
            }
            digit if self.number.len() < MAX_DIGITS => {
                if !(self.number.is_empty() && digit == "0") {
                    self.number.push_str(digit);
                }
            }
            _ => return Effect::None,
        }
        Effect::Redraw
    }

    pub fn draw<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) -> Result<(), D::Error> {
        d.clear(BACKGROUND)?;
        match &self.screen {
            Screen::Home => self.draw_home(d),
            Screen::Game => self.draw_game(d),
            Screen::Solution => self.draw_solution(d),
            Screen::Maintenance => self.draw_maintenance(d),
            Screen::Printing => {
                let game = &self.config.games[self.game];
                self.draw_message(d, &game.icon, "Impression en cours...", &self.printing)
            }
            Screen::Done => {
                self.draw_message(d, &Icon::Named("cafe".into()), "Bonne pause !", "Ton ticket est prêt.")
            }
            Screen::Failed(message) => {
                self.draw_message(d, &Icon::Named("question".into()), "Impression impossible", message)?;
                text(d, "Touche l'écran pour revenir", Point::new(240, 250), &FONT_7X13, MUTED, Alignment::Center)
            }
        }
    }

    fn draw_home<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) -> Result<(), D::Error> {
        bar(d)?;
        draw_icon(d, &Icon::Named("cafe".into()), Point::new(8, 2), 2, WHITE)?;
        text(d, &self.config.title, Point::new(48, BAR as i32 / 2), &FONT_9X18_BOLD, WHITE, Alignment::Left)?;
        if let Some(press) = self.press.filter(|p| !p.used && p.held >= HOLD_HINT_MS && title_area().contains(p.at)) {
            let width = WIDTH * press.held.min(HOLD_MS) / HOLD_MS;
            d.fill_solid(&Rectangle::new(Point::new(0, BAR as i32 - 4), Size::new(width, 4)), ACCENT)?;
        }
        if self.pages() > 1 {
            let page = format!("{}/{}", self.page + 1, self.pages());
            text(d, &page, Point::new(WIDTH as i32 - 110, BAR as i32 / 2), &FONT_8X13_BOLD, WHITE, Alignment::Center)?;
            arrow(d, page_prev(), false, if self.page > 0 { WHITE } else { MUTED })?;
            arrow(d, page_next(), true, if self.page + 1 < self.pages() { WHITE } else { MUTED })?;
        }
        let games = self.config.games.iter().enumerate().skip(self.page * PER_PAGE).take(PER_PAGE);
        for (slot, (_, game)) in games.enumerate() {
            let r = tile(slot);
            if game.is_pack() {
                card(d, r, PACK_CARD, ACCENT)?;
            } else {
                card(d, r, CARD, BORDER)?;
            }
            let icon_box = Rectangle::new(r.top_left + Point::new((r.size.width as i32 - 48) / 2, 12), Size::new(48, 48));
            draw_icon_fit(d, &game.icon, icon_box, INK)?;
            let lines = wrap(&game.title, (r.size.width as usize - 8) / 9, 2);
            let top = r.top_left.y + 78 - (lines.len() as i32 - 1) * 8;
            for (k, line) in lines.iter().enumerate() {
                let p = Point::new(r.center().x, top + k as i32 * 16);
                text(d, line, p, &FONT_9X15_BOLD, INK, Alignment::Center)?;
            }
        }
        Ok(())
    }

    fn draw_game<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) -> Result<(), D::Error> {
        let game = &self.config.games[self.game];
        bar(d)?;
        let back = back_button();
        arrow(d, Rectangle::new(back.top_left, Size::new(36, BAR)), false, WHITE)?;
        text(d, "Retour", Point::new(36, BAR as i32 / 2), &FONT_8X13_BOLD, WHITE, Alignment::Left)?;
        text(d, &game.title, Point::new(130, BAR as i32 / 2), &FONT_9X18_BOLD, WHITE, Alignment::Left)?;

        let icon_card = Rectangle::new(Point::new(12, 48), Size::new(112, 112));
        card(d, icon_card, CARD, BORDER)?;
        draw_icon_fit(d, &game.icon, Rectangle::new(Point::new(20, 56), Size::new(96, 96)), INK)?;
        if let Some(description) = &game.description {
            // Le bouton « Solution » prend le bas de la colonne.
            let lines = if game.solution.is_some() { 2 } else { 6 };
            for (k, line) in wrap(description, 16, lines).iter().enumerate() {
                text(d, line, Point::new(14, 176 + k as i32 * 15), &FONT_7X13, MUTED, Alignment::Left)?;
            }
        }

        for (i, option) in game.options.iter().enumerate() {
            self.draw_option(d, option, i, game.options.len(), self.selection[i])?;
        }
        if let Some(pack) = &game.pack {
            self.draw_pack(d, pack.pick, pack.count)?;
        }

        if game.solution.is_some() {
            let r = solution_button();
            let style = PrimitiveStyleBuilder::new().fill_color(CARD).stroke_color(ACCENT).stroke_width(2).build();
            RoundedRectangle::with_equal_corners(r, Size::new(10, 10)).into_styled(style).draw(d)?;
            text(d, "Solution", r.center(), &FONT_9X15_BOLD, ACCENT, Alignment::Center)?;
        }
        big_button(d, print_button(), "Imprimer", true)
    }

    /// Saisie du numéro de grille : champ et rappel des réglages à gauche, pavé numérique à droite.
    fn draw_solution<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) -> Result<(), D::Error> {
        let game = &self.config.games[self.game];
        bar(d)?;
        let back = back_button();
        arrow(d, Rectangle::new(back.top_left, Size::new(36, BAR)), false, WHITE)?;
        text(d, "Retour", Point::new(36, BAR as i32 / 2), &FONT_8X13_BOLD, WHITE, Alignment::Left)?;
        let title = format!("Solution · {}", game.title);
        text(d, &title, Point::new(130, BAR as i32 / 2), &FONT_9X18_BOLD, WHITE, Alignment::Left)?;

        text(d, "Numéro de la grille", Point::new(16, 54), &FONT_8X13_BOLD, INK, Alignment::Left)?;
        let field = number_field();
        card(d, field, CARD, if self.solution_number().is_some() { ACCENT } else { BORDER })?;
        let shown = if self.number.is_empty() { "n° ?" } else { &self.number };
        let ink = if self.number.is_empty() { MUTED } else { INK };
        text(d, shown, field.center(), &FONT_10X20, ink, Alignment::Center)?;

        // Les réglages doivent être ceux du ticket : on les rappelle.
        let settings: Vec<&str> = game
            .options
            .iter()
            .zip(&self.selection)
            .map(|(option, &k)| option.choices[k].label.as_str())
            .collect();
        let hint = if settings.is_empty() {
            "Le numéro est en bas à droite de la grille.".to_owned()
        } else {
            format!("Avec les réglages du ticket : {}", settings.join(", "))
        };
        for (k, line) in wrap(&hint, 29, 4).iter().enumerate() {
            text(d, line, Point::new(16, 136 + k as i32 * 15), &FONT_7X13, MUTED, Alignment::Left)?;
        }

        for (k, key) in KEYS.iter().enumerate() {
            let r = keypad_key(k);
            card(d, r, CARD, BORDER)?;
            if *key == "<" {
                arrow(d, r, false, INK)?;
            } else {
                text(d, key, r.center(), &FONT_10X20, INK, Alignment::Center)?;
            }
        }
        big_button(d, keypad_print(), "Imprimer", self.solution_number().is_some())
    }

    fn draw_option<D: DrawTarget<Color = Rgb565>>(
        &self,
        d: &mut D,
        option: &GameOption,
        index: usize,
        count: usize,
        chosen: usize,
    ) -> Result<(), D::Error> {
        let control = option_control(index, count);
        let label_at = Point::new(control.top_left.x, control.top_left.y - 10);
        text(d, &option.label, label_at, &FONT_8X13_BOLD, INK, Alignment::Left)?;
        let n = option.choices.len();
        match control_kind(n) {
            Control::Segments => {
                for (k, choice) in option.choices.iter().enumerate() {
                    let r = segment(control, k, n);
                    let (fill, ink) = if k == chosen { (ACCENT, WHITE) } else { (CARD, INK) };
                    card(d, r, fill, if k == chosen { ACCENT } else { BORDER })?;
                    let label = truncate(&choice.label, (r.size.width as usize).saturating_sub(6) / 8);
                    text(d, &label, r.center(), &FONT_8X13_BOLD, ink, Alignment::Center)?;
                }
            }
            Control::Selector => {
                for (r, right) in [(selector_prev(control), false), (selector_next(control), true)] {
                    card(d, r, ACCENT, ACCENT)?;
                    arrow(d, r, right, WHITE)?;
                }
                let middle = selector_label(control);
                card(d, middle, CARD, BORDER)?;
                let label = truncate(&option.choices[chosen].label, (middle.size.width as usize - 8) / 8);
                text(d, &label, middle.center(), &FONT_8X13_BOLD, INK, Alignment::Center)?;
            }
        }
        Ok(())
    }

    /// Colonne de droite d'un pack : ce qu'il contiendra.
    fn draw_pack<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D, pick: Pick, count: usize) -> Result<(), D::Error> {
        let pool = self.config.pack_pool(self.game);
        let count = count.min(pool.len());
        let (heading, listed): (String, Vec<usize>) = match pick {
            Pick::Tous => (format!("{} jeux au programme", pool.len()), pool),
            Pick::Hasard => (format!("{count} jeux tirés au hasard parmi :"), pool),
            Pick::Populaires => {
                let prints = |g: &usize| self.popularity.get(self.config.games[*g].key()).copied().unwrap_or(0);
                let mut ranked = pool;
                ranked.sort_by_key(|g| std::cmp::Reverse(prints(g)));
                ranked.truncate(count);
                (format!("Les {count} jeux les plus joués"), ranked)
            }
        };
        text(d, &heading, Point::new(136, 54), &FONT_8X13_BOLD, INK, Alignment::Left)?;
        let titles: Vec<&str> = listed.iter().map(|&g| self.config.games[g].title.as_str()).collect();
        // Deux colonnes de cinq lignes ; au-delà, « … et N autres ».
        let shown = if titles.len() > 10 { 9 } else { titles.len() };
        for (k, title) in titles.iter().take(shown).enumerate() {
            let at = Point::new(136 + (k / 5) as i32 * 168, 80 + (k % 5) as i32 * 24);
            text(d, &format!("- {}", truncate(title, 18)), at, &FONT_8X13, INK, Alignment::Left)?;
        }
        if titles.len() > shown {
            let more = format!("... et {} autres", titles.len() - shown);
            text(d, &more, Point::new(304, 176), &FONT_8X13, MUTED, Alignment::Left)?;
        }
        Ok(())
    }

    /// Icône Wi-Fi (30 × 24), dans le bandeau de la maintenance seulement : barrée, arcs qui
    /// s'allument tour à tour pendant la recherche, ou autant d'arcs que la force du signal.
    fn draw_wifi<D: DrawTarget<Color = Rgb565>>(
        &self,
        d: &mut D,
        at: Point,
        lit: Rgb565,
        dim: Rgb565,
        cross: Rgb565,
    ) -> Result<(), D::Error> {
        let level = match self.wifi {
            WifiStatus::Off => None,
            WifiStatus::Searching => Some((self.anim / SCAN_FRAME_MS % 4) as u8),
            WifiStatus::Connected { .. } => Some(self.wifi.bars()),
        };
        for (y, row) in WIFI_ICON.iter().enumerate() {
            for (x, b) in row.bytes().enumerate().filter(|(_, b)| b.is_ascii_digit()) {
                let on = level.is_some_and(|level| b - b'0' <= level);
                let p = at + Point::new(x as i32 * 2, y as i32 * 2);
                d.fill_solid(&Rectangle::new(p, Size::new(2, 2)), if on { lit } else { dim })?;
            }
        }
        if level.is_none() {
            let style = PrimitiveStyle::with_stroke(cross, 3);
            Line::new(at + Point::new(2, 1), at + Point::new(28, 22)).into_styled(style).draw(d)?;
        }
        Ok(())
    }

    /// Mode maintenance : horloge, Wi-Fi, et la place des futurs outils.
    fn draw_maintenance<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) -> Result<(), D::Error> {
        d.clear(M_BACKGROUND)?;
        Rectangle::new(Point::zero(), Size::new(WIDTH, BAR)).into_styled(PrimitiveStyle::with_fill(M_BAR)).draw(d)?;
        d.fill_solid(&Rectangle::new(Point::new(0, BAR as i32 - 3), Size::new(WIDTH, 3)), M_BAR_LINE)?;
        text(d, "MAINTENANCE", Point::new(12, BAR as i32 / 2 - 1), &FONT_9X18_BOLD, M_AMBER, Alignment::Left)?;
        self.draw_wifi(d, Point::new(WIDTH as i32 - 44, 5), M_TEXT, M_ICON_DIM, M_RED)?;

        let panel = |d: &mut D, r: Rectangle| {
            let style = PrimitiveStyleBuilder::new().fill_color(M_PANEL).stroke_color(M_LINE).stroke_width(1).build();
            RoundedRectangle::with_equal_corners(r, Size::new(8, 8)).into_styled(style).draw(d)
        };

        // Horloge de la carte, à comparer à une horloge de référence.
        panel(d, Rectangle::new(Point::new(12, 46), Size::new(222, 124)))?;
        text(d, "HORLOGE (RTC)", Point::new(24, 60), &FONT_8X13_BOLD, M_CYAN, Alignment::Left)?;
        // Avec le Wi-Fi connecté, la place est partagée avec la synchronisation NTP.
        let connected = matches!(self.wifi, WifiStatus::Connected { .. });
        let (time_y, date_y) = if connected { (86, 108) } else { (104, 132) };
        match &self.clock {
            Some(clock) => {
                text(d, &clock.time(), Point::new(123, time_y), &FONT_10X20, M_TEXT, Alignment::Center)?;
                text(d, &clock.date(), Point::new(123, date_y), &FONT_8X13, M_TEXT, Alignment::Center)?;
            }
            None => text(d, "Heure inconnue", Point::new(123, time_y + 8), &FONT_10X20, M_AMBER, Alignment::Center)?,
        }
        if connected {
            let (state, color) = match self.ntp {
                NtpStatus::Idle | NtpStatus::Syncing => ("Synchronisation NTP...", M_AMBER),
                NtpStatus::Synced { .. } => ("Synchronisée (NTP)", M_GREEN),
                // « Échec » : le É majuscule de cette police ressemble à un é.
                NtpStatus::Failed => ("Synchronisation échouée", M_RED),
            };
            text(d, state, Point::new(123, 134), &FONT_8X13_BOLD, color, Alignment::Center)?;
            if let NtpStatus::Synced { offset_ms } = self.ntp {
                // Écart en direct : vert sous la demi-seconde, ambre au-delà.
                let color = if offset_ms.abs() < 500 { M_GREEN } else { M_AMBER };
                let offset = format!("Écart RTC - NTP : {}", format_offset(offset_ms));
                text(d, &offset, Point::new(123, 154), &FONT_7X13, color, Alignment::Center)?;
            }
        }

        // Wi-Fi : état, et le réseau attendu.
        panel(d, Rectangle::new(Point::new(246, 46), Size::new(222, 124)))?;
        text(d, "WI-FI", Point::new(258, 60), &FONT_8X13_BOLD, M_CYAN, Alignment::Left)?;
        let wifi = &self.config.maintenance.wifi;
        match &self.wifi {
            // Connecté : le réseau et son mot de passe ne servent plus, place à la connexion.
            WifiStatus::Connected { rssi, ip } => {
                text(d, "Connecté", Point::new(258, 82), &FONT_8X13_BOLD, M_GREEN, Alignment::Left)?;
                text(d, "Signal :", Point::new(258, 108), &FONT_7X13, M_MUTED, Alignment::Left)?;
                let signal = format!("{rssi} dBm ({})", self.wifi.quality());
                text(d, &signal, Point::new(258, 122), &FONT_8X13_BOLD, M_TEXT, Alignment::Left)?;
                text(d, "Adresse :", Point::new(258, 140), &FONT_7X13, M_MUTED, Alignment::Left)?;
                let ip = ip.as_deref().unwrap_or("en attente...");
                text(d, ip, Point::new(258, 154), &FONT_8X13_BOLD, M_TEXT, Alignment::Left)?;
            }
            // Coupé ou en recherche : le réseau attendu, pour le créer au besoin.
            status => {
                let (state, color) = match status {
                    WifiStatus::Searching => ("Recherche du réseau...", M_AMBER),
                    _ => ("Désactivé", M_RED),
                };
                text(d, state, Point::new(258, 82), &FONT_8X13_BOLD, color, Alignment::Left)?;
                if wifi.ssid.is_empty() {
                    text(d, "Aucun réseau dans games.json", Point::new(258, 128), &FONT_7X13, M_AMBER, Alignment::Left)?;
                } else {
                    text(d, "Réseau attendu :", Point::new(258, 108), &FONT_7X13, M_MUTED, Alignment::Left)?;
                    text(d, &truncate(&wifi.ssid, 26), Point::new(258, 122), &FONT_8X13_BOLD, M_TEXT, Alignment::Left)?;
                    text(d, "Mot de passe :", Point::new(258, 140), &FONT_7X13, M_MUTED, Alignment::Left)?;
                    text(d, &truncate(&wifi.password, 26), Point::new(258, 154), &FONT_8X13_BOLD, M_TEXT, Alignment::Left)?;
                }
            }
        }

        // La place des outils à venir.
        for (k, label) in ["Mise à jour", "Statistiques", "Envoi des stats"].iter().enumerate() {
            let r = maintenance_slot(k);
            panel(d, r)?;
            text(d, label, r.center() - Point::new(0, 7), &FONT_8X13_BOLD, M_MUTED, Alignment::Center)?;
            text(d, "bientôt", r.center() + Point::new(0, 8), &FONT_7X13, M_LINE, Alignment::Center)?;
        }

        let version = concat!("Borne ", env!("CARGO_PKG_VERSION"));
        text(d, version, Point::new(12, HEIGHT as i32 - 30), &FONT_7X13, M_MUTED, Alignment::Left)?;
        let quit = maintenance_quit();
        let style = PrimitiveStyleBuilder::new().fill_color(M_PANEL).stroke_color(M_CYAN).stroke_width(2).build();
        RoundedRectangle::with_equal_corners(quit, Size::new(10, 10)).into_styled(style).draw(d)?;
        text(d, "Quitter", quit.center(), &FONT_10X20, M_CYAN, Alignment::Center)
    }

    fn draw_message<D: DrawTarget<Color = Rgb565>>(
        &self,
        d: &mut D,
        icon: &Icon,
        title: &str,
        detail: &str,
    ) -> Result<(), D::Error> {
        draw_icon_fit(d, icon, Rectangle::new(Point::new(200, 40), Size::new(80, 80)), INK)?;
        text(d, title, Point::new(240, 150), &FONT_10X20, INK, Alignment::Center)?;
        for (k, line) in wrap(detail, 56, 3).iter().enumerate() {
            text(d, line, Point::new(240, 182 + k as i32 * 16), &FONT_8X13, MUTED, Alignment::Center)?;
        }
        Ok(())
    }
}

// Mise en page : les mêmes rectangles servent au dessin et à la détection des appuis.

pub(crate) fn tile(slot: usize) -> Rectangle {
    let (gap, margin) = (8, 8);
    let width = (WIDTH - 2 * margin - (COLUMNS as u32 - 1) * gap) / COLUMNS as u32;
    let height = (HEIGHT - BAR - 2 * margin - (ROWS as u32 - 1) * gap) / ROWS as u32;
    let (col, row) = ((slot % COLUMNS) as u32, (slot / COLUMNS) as u32);
    let x = margin + col * (width + gap);
    let y = BAR + margin + row * (height + gap);
    Rectangle::new(Point::new(x as i32, y as i32), Size::new(width, height))
}

pub(crate) fn page_prev() -> Rectangle {
    Rectangle::new(Point::new(WIDTH as i32 - 84, 0), Size::new(40, BAR))
}

pub(crate) fn page_next() -> Rectangle {
    Rectangle::new(Point::new(WIDTH as i32 - 42, 0), Size::new(40, BAR))
}

pub(crate) fn back_button() -> Rectangle {
    Rectangle::new(Point::zero(), Size::new(110, BAR))
}

pub(crate) fn print_button() -> Rectangle {
    Rectangle::new(Point::new(WIDTH as i32 - 216, HEIGHT as i32 - 56), Size::new(200, 44))
}

/// Titre de l'accueil : un appui de 6 s ouvre le mode maintenance.
pub(crate) fn title_area() -> Rectangle {
    Rectangle::new(Point::zero(), Size::new(260, BAR))
}

pub(crate) fn maintenance_slot(k: usize) -> Rectangle {
    let width = (WIDTH - 24 - 2 * 8) / 3;
    Rectangle::new(Point::new(12 + k as i32 * (width + 8) as i32, 180), Size::new(width, 38))
}

pub(crate) fn maintenance_quit() -> Rectangle {
    Rectangle::new(Point::new(WIDTH as i32 - 172, HEIGHT as i32 - 46), Size::new(160, 38))
}

pub(crate) fn solution_button() -> Rectangle {
    Rectangle::new(Point::new(12, HEIGHT as i32 - 56), Size::new(112, 44))
}

/// Touches du pavé numérique, ligne par ligne : `C` efface tout, `<` le dernier chiffre.
const KEYS: [&str; 12] = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "C", "0", "<"];
const MAX_DIGITS: usize = 6;

pub(crate) fn keypad_key(k: usize) -> Rectangle {
    let (left, top, gap) = (236, BAR as i32 + 8, 6);
    let width = (WIDTH as i32 - 16 - left - 2 * gap) / 3;
    let height = (HEIGHT as i32 - 8 - top - 3 * gap) / 4;
    let (col, row) = ((k % 3) as i32, (k / 3) as i32);
    let at = Point::new(left + col * (width + gap), top + row * (height + gap));
    Rectangle::new(at, Size::new(width as u32, height as u32))
}

fn number_field() -> Rectangle {
    Rectangle::new(Point::new(16, 68), Size::new(204, 48))
}

pub(crate) fn keypad_print() -> Rectangle {
    Rectangle::new(Point::new(16, HEIGHT as i32 - 56), Size::new(204, 44))
}

/// Zone tactile d'un réglage (sous son libellé), dans la colonne de droite.
pub(crate) fn option_control(index: usize, count: usize) -> Rectangle {
    let (left, top, bottom) = (136, 44, HEIGHT as i32 - 64);
    let row = ((bottom - top) / count.max(1) as i32).min(66);
    let height = (row - 24).clamp(28, 40) as u32;
    Rectangle::new(Point::new(left, top + index as i32 * row + 20), Size::new(WIDTH - left as u32 - 16, height))
}

enum Control {
    /// Boutons côte à côte, jusqu'à quatre choix.
    Segments,
    /// Flèches de part et d'autre du choix courant, au-delà.
    Selector,
}

fn control_kind(choices: usize) -> Control {
    if choices <= 4 { Control::Segments } else { Control::Selector }
}

pub(crate) fn segment(control: Rectangle, k: usize, n: usize) -> Rectangle {
    let gap = 6;
    let width = (control.size.width - (n as u32 - 1) * gap) / n as u32;
    let x = control.top_left.x + (k as u32 * (width + gap)) as i32;
    Rectangle::new(Point::new(x, control.top_left.y), Size::new(width, control.size.height))
}

pub(crate) fn selector_prev(control: Rectangle) -> Rectangle {
    Rectangle::new(control.top_left, Size::new(48, control.size.height))
}

pub(crate) fn selector_next(control: Rectangle) -> Rectangle {
    let x = control.top_left.x + control.size.width as i32 - 48;
    Rectangle::new(Point::new(x, control.top_left.y), Size::new(48, control.size.height))
}

pub(crate) fn selector_label(control: Rectangle) -> Rectangle {
    let x = control.top_left.x + 54;
    Rectangle::new(Point::new(x, control.top_left.y), Size::new(control.size.width - 108, control.size.height))
}

// Dessin.

/// Bouton principal (« Imprimer »), grisé s'il n'est pas utilisable.
fn big_button<D: DrawTarget<Color = Rgb565>>(d: &mut D, r: Rectangle, label: &str, enabled: bool) -> Result<(), D::Error> {
    let fill = if enabled { ACCENT } else { BORDER };
    RoundedRectangle::with_equal_corners(r, Size::new(10, 10)).into_styled(PrimitiveStyle::with_fill(fill)).draw(d)?;
    text(d, label, r.center(), &FONT_10X20, WHITE, Alignment::Center)
}

fn bar<D: DrawTarget<Color = Rgb565>>(d: &mut D) -> Result<(), D::Error> {
    Rectangle::new(Point::zero(), Size::new(WIDTH, BAR)).into_styled(PrimitiveStyle::with_fill(BAR_COLOR)).draw(d)
}

fn card<D: DrawTarget<Color = Rgb565>>(d: &mut D, r: Rectangle, fill: Rgb565, stroke: Rgb565) -> Result<(), D::Error> {
    let style = PrimitiveStyleBuilder::new().fill_color(fill).stroke_color(stroke).stroke_width(1).build();
    RoundedRectangle::with_equal_corners(r, Size::new(8, 8)).into_styled(style).draw(d)
}

/// Flèche (triangle) centrée dans `r`, vers la droite ou vers la gauche.
fn arrow<D: DrawTarget<Color = Rgb565>>(d: &mut D, r: Rectangle, right: bool, color: Rgb565) -> Result<(), D::Error> {
    let c = r.center();
    let (tip, base) = if right { (8, -6) } else { (-8, 6) };
    Triangle::new(Point::new(c.x + tip, c.y), Point::new(c.x + base, c.y - 9), Point::new(c.x + base, c.y + 9))
        .into_styled(PrimitiveStyle::with_fill(color))
        .draw(d)
}

fn text<D: DrawTarget<Color = Rgb565>>(
    d: &mut D,
    s: &str,
    at: Point,
    font: &MonoFont<'_>,
    color: Rgb565,
    alignment: Alignment,
) -> Result<(), D::Error> {
    let style = TextStyleBuilder::new().alignment(alignment).baseline(Baseline::Middle).build();
    Text::with_text_style(s, at, MonoTextStyle::new(font, color), style).draw(d)?;
    Ok(())
}

fn draw_icon<D: DrawTarget<Color = Rgb565>>(
    d: &mut D,
    icon: &Icon,
    at: Point,
    scale: u32,
    color: Rgb565,
) -> Result<(), D::Error> {
    let (w, h) = icon.size();
    let points = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| icon.is_on(x, y));
    for (x, y) in points {
        let p = at + Point::new((x as u32 * scale) as i32, (y as u32 * scale) as i32);
        d.fill_solid(&Rectangle::new(p, Size::new(scale, scale)), color)?;
    }
    Ok(())
}

/// Icône agrandie au plus grand facteur entier qui tient dans `area`, centrée.
fn draw_icon_fit<D: DrawTarget<Color = Rgb565>>(
    d: &mut D,
    icon: &Icon,
    area: Rectangle,
    color: Rgb565,
) -> Result<(), D::Error> {
    let (w, h) = icon.size();
    let side = w.max(h).max(1) as u32;
    let scale = (area.size.width.min(area.size.height) / side).max(1);
    let offset = Point::new(
        (area.size.width as i32 - (w as u32 * scale) as i32) / 2,
        (area.size.height as i32 - (h as u32 * scale) as i32) / 2,
    );
    draw_icon(d, icon, area.top_left + offset, scale, color)
}

/// Coupe un texte aux espaces en lignes d'au plus `width` caractères, `max` lignes au plus.
fn wrap(s: &str, width: usize, max: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in s.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + word.chars().count() <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_owned()),
        }
    }
    if lines.len() > max {
        lines.truncate(max);
        let last = lines.last_mut().expect("au moins une ligne");
        *last = truncate(&format!("{last}..."), width);
    }
    lines.iter().map(|l| truncate(l, width)).collect()
}

fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        return s.to_owned();
    }
    let kept: String = s.chars().take(width.saturating_sub(1)).collect();
    format!("{kept}.")
}

#[cfg(test)]
pub(crate) mod layout {
    //! Accès à la mise en page pour les tests.
    pub(crate) use super::{
        back_button, keypad_key, keypad_print, maintenance_quit, option_control, page_next, print_button, segment,
        selector_next, solution_button, tile, title_area,
    };
}
