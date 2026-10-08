//! Simulateur de la borne : joue une suite d'appuis sur l'interface, enregistre chaque écran en
//! PNG et écrit les tickets demandés (au format Printr), sans écran ni imprimante.
//!
//! ```sh
//! cargo run -p borne-sim                                   # visite guidée, captures dans kiosk/captures
//! cargo run -p borne-sim -- tap:299,97 tap:364,238         # ses propres appuis (x,y en points)
//! cargo run -p borne-sim -- tap:299,97 tap:364,238 | cargo run -- --preview print
//! ```
//!
//! Étapes : `tap:X,Y` (appui), `wait:MS` (temps qui passe), `fail` (la prochaine impression échoue).
//! Options : `--kiosk <dossier>` (games.json et templates/, défaut `kiosk`), `--out <dossier>`, et
//! `--seed <nombre>` pour rejouer le même tirage des packs (sinon tiré de l'horloge), et
//! `--glitch <n>` pour un ticket glitch une impression sur `n` (`1` : à chaque fois, `0` : jamais).

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use borne_ui::{App, Config, Effect, Event, Framebuffer, Point, HEIGHT, WIDTH};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::RgbColor;

/// Visite guidée : pages, pack au hasard imprimé, sudoku réglé et imprimé, pack des plus joués,
/// mots mêlés, puis la solution d'un sudoku tapée sur le pavé numérique.
const TOUR: &[&str] = &[
    "tap:458,18",  // page suivante
    "tap:416,18",  // page précédente
    "tap:63,97",   // Pack pause
    "tap:364,238", // Imprimer
    "wait:6000",   // retour à l'accueil
    "tap:299,97",  // Sudoku
    "tap:410,84",  // niveau difficile
    "tap:364,238", // Imprimer
    "wait:6000",   // retour à l'accueil
    "tap:181,97",  // Les plus joués (le sudoku vient d'être imprimé)
    "tap:55,18",   // Retour
    "tap:417,97",  // Mots mêlés
    "tap:440,150", // thème suivant
    "tap:55,18",   // Retour
    "tap:299,97",  // Sudoku
    "tap:68,238",  // Solution
    "tap:272,125", // 4
    "tap:350,69",  // 2
    "tap:272,69",  // 1
    "tap:350,125", // 5
    "tap:118,238", // Imprimer la solution
    "wait:6000",   // retour à l'accueil
];

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut kiosk = PathBuf::from("kiosk");
    let mut out = PathBuf::from("kiosk/captures");
    let mut steps = Vec::new();
    let mut glitch = None;
    let mut seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos() as u64;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--kiosk" => kiosk = args.next().context("--kiosk attend un dossier")?.into(),
            "--out" => out = args.next().context("--out attend un dossier")?.into(),
            "--seed" => seed = args.next().context("--seed attend un nombre")?.parse()?,
            "--glitch" => glitch = Some(args.next().context("--glitch attend un nombre")?.parse()?),
            _ => steps.push(arg),
        }
    }
    if steps.is_empty() {
        steps = TOUR.iter().map(|s| s.to_string()).collect();
    }

    let config = Config::load(&kiosk).map_err(anyhow::Error::msg)?;
    std::fs::create_dir_all(&out)?;

    let mut app = App::new(config);
    app.seed(seed);
    if let Some(odds) = glitch {
        app.set_glitch_odds(odds);
    }
    let mut frame = Framebuffer::new();
    let mut shots = 0;
    let mut tickets = 0;
    let mut fail_next = false;
    let mut capture = |app: &App, name: &str| -> Result<()> {
        app.draw(&mut frame).expect("dessin en mémoire");
        let path = out.join(format!("{shots:02}-{name}.png"));
        save_png(&frame, &path)?;
        eprintln!("{}", path.display());
        shots += 1;
        Ok(())
    };

    capture(&app, "accueil")?;
    for step in &steps {
        let event = match step.split_once(':') {
            Some(("tap", xy)) => {
                let (x, y) = xy.split_once(',').context("tap:X,Y")?;
                Event::Tap(Point::new(x.trim().parse()?, y.trim().parse()?))
            }
            Some(("wait", ms)) => Event::Tick(ms.trim().parse()?),
            None if step == "fail" => {
                fail_next = true;
                continue;
            }
            _ => bail!("étape inconnue « {step} » (tap:X,Y, wait:MS ou fail)"),
        };
        let name = step.replace([':', ','], "-");
        match app.handle(event) {
            Effect::None => {}
            Effect::Redraw => capture(&app, &name)?,
            Effect::Print(printed) => {
                capture(&app, &format!("{name}-impression"))?;
                // Un fichier et une ligne par ticket (un glitch peut précéder le ticket demandé).
                for ticket in printed {
                    tickets += 1;
                    let path = out.join(format!("ticket-{tickets}.json"));
                    std::fs::write(&path, &ticket)?;
                    eprintln!("{}", path.display());
                    println!("{ticket}");
                }
                let result = if std::mem::take(&mut fail_next) { Err("imprimante injoignable".to_owned()) } else { Ok(()) };
                if app.print_finished(result) == Effect::Redraw {
                    capture(&app, &format!("{name}-fin"))?;
                }
            }
        }
    }
    Ok(())
}

fn save_png(frame: &Framebuffer, path: &PathBuf) -> Result<()> {
    let mut img = image::RgbImage::new(WIDTH, HEIGHT);
    for (pixel, &color) in img.pixels_mut().zip(frame.pixels()) {
        let c = Rgb888::from(color);
        *pixel = image::Rgb([c.r(), c.g(), c.b()]);
    }
    img.save(path).with_context(|| format!("impossible d'écrire {}", path.display()))
}
