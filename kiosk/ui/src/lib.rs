//! Interface tactile de la borne de jeux du FabLab, indépendante du matériel.
//!
//! - [`Config`] : la description de la borne (`games.json`) et les tickets qu'elle produit ;
//! - [`App`] : les écrans, à qui l'on transmet les appuis et le temps qui passe ;
//! - [`Framebuffer`] : une image de l'écran en mémoire, à envoyer à l'afficheur.

mod app;
mod config;
mod icons;
mod status;

use std::convert::Infallible;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

pub use app::{App, Effect, Event, HEIGHT, WIDTH};
pub use config::{
    Choice, Config, Game, GameOption, Icon, Maintenance, Pack, Pick, Solution, WifiNetwork, DEFAULT_TEMPLATE, PLACEHOLDER,
};
pub use status::{format_offset, Clock, NtpStatus, WifiStatus};
pub use embedded_graphics::prelude::Point;

/// Image de l'écran en mémoire (RGB565), dans laquelle [`App::draw`] dessine.
pub struct Framebuffer {
    pixels: Vec<Rgb565>,
}

impl Framebuffer {
    pub fn new() -> Self {
        Self { pixels: vec![Rgb565::BLACK; (WIDTH * HEIGHT) as usize] }
    }

    /// Points ligne par ligne, de gauche à droite et de haut en bas.
    pub fn pixels(&self) -> &[Rgb565] {
        &self.pixels
    }
}

impl Default for Framebuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl OriginDimensions for Framebuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Framebuffer {
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I: IntoIterator<Item = Pixel<Rgb565>>>(&mut self, pixels: I) -> Result<(), Infallible> {
        for Pixel(p, color) in pixels {
            if (0..WIDTH as i32).contains(&p.x) && (0..HEIGHT as i32).contains(&p.y) {
                self.pixels[(p.y as u32 * WIDTH + p.x as u32) as usize] = color;
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Rgb565) -> Result<(), Infallible> {
        self.pixels.fill(color);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::app::layout;
    use super::*;
    use serde_json::{json, Value};

    fn kiosk() -> Config {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        Config::load(&dir).expect("kiosk/ valide")
    }

    fn app() -> App {
        App::new(kiosk())
    }

    fn tap(app: &mut App, r: embedded_graphics::primitives::Rectangle) -> Effect {
        app.handle(Event::Tap(r.center()))
    }

    /// Le ticket demandé (le dernier de l'impression).
    fn printed(effect: Effect) -> String {
        match effect {
            Effect::Print(mut tickets) => tickets.pop().expect("au moins un ticket"),
            other => panic!("impression attendue, obtenu {other:?}"),
        }
    }

    #[test]
    fn games_json_is_valid_and_renders() {
        let config = kiosk();
        assert!(config.games.len() >= 8);
        let mut app = App::new(config);
        let mut frame = Framebuffer::new();
        app.draw(&mut frame).unwrap();
        // Chaque jeu s'ouvre et se dessine.
        for game in 0..app.config().games.len() {
            let mut app2 = self::app();
            if game >= 8 {
                tap(&mut app2, layout::page_next());
            }
            assert_eq!(tap(&mut app2, layout::tile(game % 8)), Effect::Redraw);
            app2.draw(&mut frame).unwrap();
        }
        app.handle(Event::Tick(10));
    }

    #[test]
    fn printing_flow_builds_the_ticket_and_comes_back_home() {
        let mut app = app();
        tap(&mut app, layout::tile(2)); // Sudoku
        let ticket = printed(tap(&mut app, layout::print_button()));
        let ticket: Value = serde_json::from_str(&ticket).unwrap();
        let blocks = ticket["blocks"].as_array().unwrap();
        assert!(blocks.iter().any(|b| b["type"] == "sudoku" && b["difficulty"] == "moyen"));
        assert_eq!(app.print_finished(Ok(())), Effect::Redraw);
        assert_eq!(app.handle(Event::Tick(6_000)), Effect::Redraw);
        // De retour à l'accueil : un appui sur une tuile rouvre un jeu.
        assert_eq!(tap(&mut app, layout::tile(0)), Effect::Redraw);
    }

    #[test]
    fn options_change_the_ticket() {
        let mut app = app();
        tap(&mut app, layout::tile(2)); // Sudoku
        // Premier réglage du sudoku : la difficulté, en trois segments ; on prend « difficile ».
        let control = layout::option_control(0, 1);
        app.handle(Event::Tap(layout::segment(control, 2, 3).center()));
        let ticket = printed(tap(&mut app, layout::print_button()));
        assert!(ticket.contains(r#""difficulty":"difficile""#), "{ticket}");
    }

    #[test]
    fn null_choice_removes_the_parameter_and_objects_are_merged() {
        let config = Config::from_json(
            r#"{"title": "Test", "games": [{"title": "Jeu", "icon": "sudoku",
                "block": {"type": "maze", "width": 12, "height": 16, "seed": 3},
                "options": [
                  {"label": "Graine", "key": "seed", "choices": [{"label": "Au hasard", "value": null}]},
                  {"label": "Taille", "choices": [
                    {"label": "Petit", "value": {"width": 8, "height": 10}},
                    {"label": "Moyen", "value": {"width": 12, "height": 16}}]}
                ]}]}"#,
        )
        .unwrap();
        assert_eq!(config.default_selection(0), [0, 1]);
        let ticket = config.ticket(0, &[0, 0]);
        assert_eq!(ticket["blocks"][0], json!({"type": "maze", "width": 8, "height": 10}));
    }

    #[test]
    fn rejects_inconsistent_configs() {
        let bad = [
            r#"{"title": "T", "games": []}"#,
            r#"{"title": "T", "games": [{"title": "J", "icon": "sudoku", "block": {}}]}"#,
            r#"{"title": "T", "games": [{"title": "J", "icon": "licorne", "block": {"type": "sudoku"}}]}"#,
            r##"{"title": "T", "games": [{"title": "J", "icon": ["#.", "#"], "block": {"type": "sudoku"}}]}"##,
            r#"{"title": "T", "games": [{"title": "J", "icon": "sudoku", "block": {"type": "sudoku"},
                "options": [{"label": "O", "choices": [{"label": "a", "value": 1}]}]}]}"#,
        ];
        for json in bad {
            assert!(Config::from_json(json).is_err(), "{json}");
        }
    }

    #[test]
    fn templates_frame_the_game() {
        let games = r#"{"title": "T", "template": "pause", "games": [
            {"title": "A", "icon": "sudoku", "block": {"type": "sudoku"}},
            {"title": "B", "icon": "fleur", "block": {"type": "coloriage"}, "template": "dessin"}]}"#;
        let pause = r#"{"cut": false, "spacing": 2, "blocks": [{"type": "title", "text": "Pause"}, {"type": "jeu"}, {"type": "date"}]}"#;
        let dessin = r#"{"blocks": [{"type": "jeu"}]}"#;
        let config = Config::from_parts(games, [("pause".into(), pause.into()), ("dessin".into(), dessin.into())]).unwrap();
        assert_eq!(
            config.ticket(0, &[]),
            json!({"cut": false, "spacing": 2, "blocks": [{"type": "title", "text": "Pause"}, {"type": "sudoku"}, {"type": "date"}]})
        );
        assert_eq!(config.ticket(1, &[]), json!({"blocks": [{"type": "coloriage"}]}));
        // Sans modèle « defaut », le ticket ne contient que le jeu.
        let bare = Config::from_json(r#"{"title": "T", "games": [{"title": "A", "icon": "sudoku", "block": {"type": "sudoku"}}]}"#);
        assert_eq!(bare.unwrap().ticket(0, &[]), json!({"blocks": [{"type": "sudoku"}]}));
    }

    #[test]
    fn rejects_bad_templates() {
        let games = r#"{"title": "T", "template": "pause", "games": [{"title": "A", "icon": "sudoku", "block": {"type": "sudoku"}}]}"#;
        assert!(Config::from_parts(games, []).is_err(), "modèle introuvable");
        for template in [r#"{"blocks": [{"type": "date"}]}"#, r#"{"blocks": [{"type": "jeu"}, {"type": "jeu"}]}"#, "pas du json"] {
            assert!(Config::from_parts(games, [("pause".into(), template.into())]).is_err(), "{template}");
        }
    }

    const PACKS: &str = r#"{"title": "T", "games": [
        {"title": "Hasard", "icon": "cafe", "pack": {"pick": "hasard", "count": 3}},
        {"title": "Populaires", "icon": "coeur", "pack": {"pick": "populaires", "count": 2, "games": ["a", "b", "c"]}},
        {"title": "Duo", "icon": "cafe", "pack": {"games": ["c", "a"]}, "template": "pack"},
        {"id": "a", "title": "A", "icon": "sudoku", "block": {"type": "sudoku"}},
        {"id": "b", "title": "B", "icon": "loupe", "block": {"type": "mots_meles"}},
        {"id": "c", "title": "C", "icon": "rails", "block": {"type": "voie_ferree"}},
        {"id": "d", "title": "D", "icon": "fleur", "block": {"type": "coloriage"}}]}"#;

    fn packs() -> App {
        let template = r#"{"blocks": [{"type": "title", "text": "Pack"}, {"type": "jeu", "entre": [{"type": "separator"}]}]}"#;
        App::new(Config::from_parts(PACKS, [("pack".into(), template.into())]).unwrap())
    }

    #[test]
    fn random_pack_draws_distinct_games() {
        let mut app = packs();
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0..20 {
            app.seed(seed);
            let games = app.pack_games(0);
            assert_eq!(games.len(), 3);
            assert!(games.iter().all(|&g| (3..=6).contains(&g)), "jamais un pack : {games:?}");
            let distinct: std::collections::BTreeSet<_> = games.iter().collect();
            assert_eq!(distinct.len(), 3);
            seen.insert(games);
        }
        assert!(seen.len() > 1, "le tirage change d'un ticket à l'autre");
    }

    #[test]
    fn popular_pack_follows_print_counts() {
        let mut app = packs();
        // Sans statistiques : l'ordre de games.json.
        assert_eq!(app.pack_games(1), [3, 4]);
        app.set_popularity([("c".to_owned(), 5), ("b".to_owned(), 2), ("d".to_owned(), 9)].into());
        // « d » est le plus imprimé, mais hors de la liste du pack.
        assert_eq!(app.pack_games(1), [5, 4]);
    }

    #[test]
    fn excluded_games_are_never_drawn() {
        let games = r#"{"title": "T", "games": [
            {"title": "P", "icon": "cafe", "pack": {"pick": "hasard", "count": 9, "exclude": ["b"]}},
            {"title": "Vide", "icon": "cafe", "pack": {"games": ["a"], "exclude": ["a"]}},
            {"id": "a", "title": "A", "icon": "sudoku", "block": {"type": "sudoku"}},
            {"id": "b", "title": "B", "icon": "loupe", "block": {"type": "petit_bac"}}]}"#;
        assert!(Config::from_json(games).is_err(), "un pack sans aucun jeu est refusé");
        let games = games.replace(r#"{"title": "Vide", "icon": "cafe", "pack": {"games": ["a"], "exclude": ["a"]}},"#, "");
        let mut app = App::new(Config::from_json(&games).unwrap());
        assert_eq!(app.pack_games(0), [1]);
        let bad = games.replace(r#""exclude": ["b"]"#, r#""exclude": ["zzz"]"#);
        assert!(Config::from_json(&bad).is_err(), "id inconnu dans exclude");
    }

    #[test]
    fn printing_a_game_counts_for_popularity() {
        let mut app = App::new(kiosk());
        tap(&mut app, layout::tile(2)); // Sudoku
        tap(&mut app, layout::print_button());
        assert_eq!(app.popularity().get("sudoku"), Some(&1));
    }

    #[test]
    fn pack_ticket_puts_games_in_the_template() {
        let mut app = packs();
        let games = app.pack_games(2);
        assert_eq!(games, [5, 3]);
        assert_eq!(
            app.config().pack_ticket(2, &games),
            json!({"blocks": [{"type": "title", "text": "Pack"}, {"type": "voie_ferree"}, {"type": "separator"}, {"type": "sudoku"}]})
        );
        // Depuis l'écran : la tuile du pack, puis « Imprimer ».
        tap(&mut app, layout::tile(2));
        let ticket = printed(tap(&mut app, layout::print_button()));
        assert!(ticket.contains("voie_ferree") && ticket.contains("sudoku"), "{ticket}");
    }

    #[test]
    fn rejects_bad_packs() {
        let bad = [
            r#"{"title": "T", "games": [{"title": "P", "icon": "cafe", "pack": {"games": ["x"]}}]}"#,
            r#"{"title": "T", "games": [{"title": "P", "icon": "cafe", "pack": {"count": 0}}]}"#,
            r#"{"title": "T", "games": [{"title": "P", "icon": "cafe", "pack": {}, "block": {"type": "sudoku"}}]}"#,
            r#"{"title": "T", "games": [{"title": "P", "icon": "cafe", "pack": {},
                "options": [{"label": "O", "key": "k", "choices": [{"label": "a", "value": 1}]}]}]}"#,
            r#"{"title": "T", "games": [{"id": "a", "title": "A", "icon": "cafe", "block": {"type": "sudoku"}},
                {"id": "a", "title": "B", "icon": "cafe", "block": {"type": "sudoku"}}]}"#,
        ];
        for json in bad {
            assert!(Config::from_json(json).is_err(), "{json}");
        }
        let template = r#"{"blocks": [{"type": "jeu", "nombre": 3}]}"#;
        assert!(Config::from_parts(PACKS, [("pack".into(), template.into())]).is_err());
    }

    /// Tape un numéro sur le pavé (touches 1-9 aux indices 0-8, 0 à l'indice 10).
    fn type_number(app: &mut App, number: &str) {
        for digit in number.bytes() {
            let key = if digit == b'0' { 10 } else { (digit - b'1') as usize };
            assert_eq!(tap(app, layout::keypad_key(key)), Effect::Redraw);
        }
    }

    #[test]
    fn solution_uses_the_typed_number_and_the_settings() {
        let mut app = app();
        tap(&mut app, layout::tile(2)); // Sudoku
        app.handle(Event::Tap(layout::segment(layout::option_control(0, 1), 2, 3).center())); // difficile
        assert_eq!(tap(&mut app, layout::solution_button()), Effect::Redraw);
        // Rien à imprimer tant qu'aucun numéro n'est saisi.
        assert_eq!(tap(&mut app, layout::keypad_print()), Effect::None);
        type_number(&mut app, "42159");
        tap(&mut app, layout::keypad_key(11)); // efface le 9
        let ticket = printed(tap(&mut app, layout::keypad_print()));
        let ticket: Value = serde_json::from_str(&ticket).unwrap();
        let sudoku = ticket["blocks"].as_array().unwrap().iter().find(|b| b["type"] == "sudoku").unwrap();
        assert_eq!(sudoku, &json!({"type": "sudoku", "difficulty": "difficile", "seed": 4215, "solution": true}));
        // Une solution ne compte pas pour la popularité.
        assert!(app.popularity().is_empty());
    }

    #[test]
    fn solution_number_respects_the_maximum() {
        let mut app = app();
        tap(&mut app, layout::tile(5)); // Logimage : numéros 1 à 10
        tap(&mut app, layout::solution_button());
        type_number(&mut app, "11");
        assert_eq!(tap(&mut app, layout::keypad_print()), Effect::None);
        tap(&mut app, layout::keypad_key(9)); // C
        type_number(&mut app, "10");
        let ticket = printed(tap(&mut app, layout::keypad_print()));
        assert!(ticket.contains(r#""number":10"#) && ticket.contains(r#""solution":true"#), "{ticket}");
    }

    #[test]
    fn games_without_solution_have_no_button() {
        let mut app = app();
        tap(&mut app, layout::tile(6)); // Labyrinthe
        assert_eq!(tap(&mut app, layout::solution_button()), Effect::None);
        tap(&mut app, layout::back_button());
        tap(&mut app, layout::tile(0)); // un pack non plus
        assert_eq!(tap(&mut app, layout::solution_button()), Effect::None);
    }

    #[test]
    fn glitch_is_its_own_ticket_and_rare() {
        // Toujours, pour vérifier sa place : un ticket à part, coupé, avant le ticket demandé.
        let mut app = self::app();
        app.set_glitch_odds(1);
        tap(&mut app, layout::tile(2)); // Sudoku
        let Effect::Print(tickets) = tap(&mut app, layout::print_button()) else { panic!() };
        assert_eq!(tickets.len(), 2);
        let glitch: Value = serde_json::from_str(&tickets[0]).unwrap();
        assert_eq!(glitch, json!({"cut": true, "blocks": [{"type": "glitch"}]}));
        assert!(!tickets[1].contains("glitch") && tickets[1].contains("sudoku"));
        // Fréquence : à un sur 100, environ 200 glitchs en 20 000 impressions.
        let mut app = self::app();
        app.seed(42);
        app.set_glitch_odds(100);
        let mut glitches = 0;
        for _ in 0..20_000 {
            tap(&mut app, layout::tile(2));
            let Effect::Print(tickets) = tap(&mut app, layout::print_button()) else { panic!() };
            glitches += tickets.len() - 1;
            app.print_finished(Ok(()));
            tap(&mut app, layout::back_button()); // écran « Bonne pause » : retour à l'accueil
        }
        assert!((140..=260).contains(&glitches), "{glitches} glitchs");
        // Par défaut : un sur 100 000 (0,001 %).
        assert_eq!(self::app().glitch_odds(), 100_000);
        // 0 : jamais.
        let mut app = self::app();
        app.set_glitch_odds(0);
        tap(&mut app, layout::tile(2));
        assert_eq!(tap(&mut app, layout::print_button()), Effect::Print(vec![app.config().ticket(2, &[1]).to_string()]));
    }

    /// Appui long : doigt posé, temps qui passe par pas de 100 ms, doigt levé.
    fn hold(app: &mut App, at: Point, ms: u32) -> Vec<Effect> {
        app.handle(Event::Down(at));
        let effects = (0..ms / 100).map(|_| app.handle(Event::Tick(100))).collect();
        app.handle(Event::Up);
        effects
    }

    #[test]
    fn long_press_on_the_title_opens_maintenance() {
        let mut app = app();
        let title = layout::title_area().center();
        // 5 s ne suffisent pas, et lever le doigt efface la barre de progression.
        app.handle(Event::Down(title));
        for _ in 0..50 {
            app.handle(Event::Tick(100));
        }
        assert_eq!(app.handle(Event::Up), Effect::Redraw);
        assert!(!app.wifi_wanted());
        // 6 s : le mode maintenance s'ouvre, et la carte doit chercher le Wi-Fi.
        let effects = hold(&mut app, title, 6_000);
        assert_eq!(effects.iter().filter(|e| **e == Effect::Redraw).count(), 51, "progression puis ouverture");
        assert!(app.wifi_wanted());
        // Le lever du doigt n'est pas un appui : on reste en maintenance, même longtemps après.
        app.handle(Event::Tick(3_600_000));
        assert!(app.wifi_wanted());
        assert_eq!(app.config().maintenance.wifi.ssid, "FabLab-Maintenance");
        // « Quitter » : retour à l'accueil, Wi-Fi coupé.
        assert_eq!(tap(&mut app, layout::maintenance_quit()), Effect::Redraw);
        assert!(!app.wifi_wanted());
    }

    #[test]
    fn long_press_elsewhere_does_nothing() {
        let mut app = app();
        hold(&mut app, layout::tile(2).center(), 7_000); // sur une tuile : simple appui au lever
        assert!(!app.wifi_wanted());
        // Sur l'écran du sudoku ouvert par ce lever, la même zone est « Retour » : un simple appui.
        hold(&mut app, layout::title_area().center(), 7_000);
        assert!(!app.wifi_wanted());
        assert_eq!(tap(&mut app, layout::tile(2)), Effect::Redraw, "de retour à l'accueil");
    }

    #[test]
    fn down_up_is_a_tap() {
        let mut app = app();
        app.handle(Event::Down(layout::tile(2).center()));
        assert_eq!(app.handle(Event::Up), Effect::Redraw);
        assert_eq!(app.handle(Event::Up), Effect::None, "un seul appui par doigt posé");
    }

    /// Position dans l'image courante de l'animation Wi-Fi (0 à 399 ms).
    fn app_anim_phase(app: &App) -> u32 {
        app.anim_ms() % 400
    }

    #[test]
    fn wifi_status_and_clock_redraw_when_needed() {
        let mut app = app();
        let clock = Clock { year: 2026, month: 10, day: 8, hour: 22, minute: 31, second: 5 };
        assert_eq!(app.set_clock(clock), Effect::None, "l'heure ne s'affiche qu'en maintenance");
        assert_eq!(app.set_wifi(WifiStatus::Searching), Effect::Redraw);
        assert_eq!(app.set_wifi(WifiStatus::Searching), Effect::None);
        // Hors maintenance, pas d'icône Wi-Fi : son animation ne redessine rien.
        assert_eq!(app.handle(Event::Tick(400)), Effect::None);
        hold(&mut app, layout::title_area().center(), 6_000);
        // En maintenance, la recherche anime l'icône toutes les 400 ms.
        app.handle(Event::Tick(400 - app_anim_phase(&app)));
        assert_eq!(app.handle(Event::Tick(399)), Effect::None);
        assert_eq!(app.handle(Event::Tick(1)), Effect::Redraw);
        assert_eq!(app.set_clock(Clock { second: 6, ..clock }), Effect::Redraw);
        assert_eq!(app.set_wifi(WifiStatus::Connected { rssi: -58, ip: Some("192.168.4.27".into()) }), Effect::Redraw);
        app.draw(&mut Framebuffer::new()).unwrap();
    }

    #[test]
    fn ntp_only_once_connected_in_maintenance() {
        let mut app = app();
        app.set_wifi(WifiStatus::Connected { rssi: -50, ip: None });
        assert!(!app.ntp_wanted(), "pas hors maintenance");
        assert_eq!(app.set_ntp(NtpStatus::Syncing), Effect::None, "rien à redessiner hors maintenance");
        hold(&mut app, layout::title_area().center(), 6_000);
        assert!(app.ntp_wanted());
        assert_eq!(app.set_ntp(NtpStatus::Synced { offset_ms: 12 }), Effect::Redraw);
        assert_eq!(app.set_ntp(NtpStatus::Synced { offset_ms: 13 }), Effect::Redraw, "l'écart se met à jour");
        app.draw(&mut Framebuffer::new()).unwrap();
        // Wi-Fi perdu : plus de synchronisation demandée.
        app.set_wifi(WifiStatus::Searching);
        assert!(!app.ntp_wanted());
        // En quittant, l'état est oublié : nouvelle synchronisation à la prochaine maintenance.
        tap(&mut app, layout::maintenance_quit());
        assert_eq!(app.ntp(), NtpStatus::Idle);
    }

    #[test]
    fn idle_kiosk_returns_home() {
        let mut app = app();
        tap(&mut app, layout::tile(1));
        assert_eq!(app.handle(Event::Tick(30_000)), Effect::None);
        assert_eq!(app.handle(Event::Tick(30_000)), Effect::Redraw);
        assert_eq!(tap(&mut app, layout::back_button()), Effect::None); // déjà à l'accueil
    }

    #[test]
    fn selector_cycles_through_many_choices() {
        let mut app = app();
        tap(&mut app, layout::tile(3)); // Mots mêlés : difficulté puis thème (plus de quatre choix)
        let control = layout::option_control(1, 2);
        assert_eq!(app.handle(Event::Tap(layout::selector_next(control).center())), Effect::Redraw);
        let ticket = printed(tap(&mut app, layout::print_button()));
        assert!(ticket.contains(r#""theme":"animaux""#), "{ticket}");
    }
}
