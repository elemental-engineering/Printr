//! Mots mêlés (mots cachés) : des mots français cachés dans une grille de lettres.
//! Le numéro imprimé est la graine : le même numéro avec `solution: true` imprime la solution.

use rand::rngs::StdRng;
use rand::seq::{IndexedRandom, SliceRandom};
use rand::{RngExt, SeedableRng};
use serde::Deserialize;

use super::sudoku::Difficulty;
use crate::doc::{Align, Doc, Style};
use crate::draw;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Animaux,
    #[serde(alias = "fruits", alias = "legumes")]
    FruitsLegumes,
    Cuisine,
    Nature,
    Sport,
    Metiers,
    Maison,
    Voyage,
    Musique,
    Ecole,
    #[serde(alias = "dev", alias = "developpement_logiciel")]
    Developpement,
    Devops,
    #[serde(alias = "reseau", alias = "networking")]
    Reseaux,
    #[serde(alias = "ai", alias = "intelligence_artificielle")]
    Ia,
}

pub(crate) const THEMES: [Theme; 14] = [
    Theme::Animaux,
    Theme::FruitsLegumes,
    Theme::Cuisine,
    Theme::Nature,
    Theme::Sport,
    Theme::Metiers,
    Theme::Maison,
    Theme::Voyage,
    Theme::Musique,
    Theme::Ecole,
    Theme::Developpement,
    Theme::Devops,
    Theme::Reseaux,
    Theme::Ia,
];

impl Theme {
    pub fn label(self) -> &'static str {
        match self {
            Theme::Animaux => "animaux",
            Theme::FruitsLegumes => "fruits et légumes",
            Theme::Cuisine => "cuisine",
            Theme::Nature => "nature",
            Theme::Sport => "sport",
            Theme::Metiers => "métiers",
            Theme::Maison => "maison",
            Theme::Voyage => "voyage",
            Theme::Musique => "musique",
            Theme::Ecole => "école",
            Theme::Developpement => "développement",
            Theme::Devops => "DevOps",
            Theme::Reseaux => "réseaux",
            Theme::Ia => "intelligence artificielle",
        }
    }

    pub(crate) fn words(self) -> &'static [&'static str] {
        match self {
            Theme::Animaux => &[
                "chat", "chien", "lapin", "cheval", "vache", "mouton", "cochon", "canard", "poule", "renard",
                "loup", "ours", "tigre", "lion", "girafe", "zèbre", "singe", "hibou", "aigle", "baleine",
                "dauphin", "tortue", "grenouille", "écureuil", "hérisson", "souris", "chèvre", "panda", "koala",
                "castor",
            ],
            Theme::FruitsLegumes => &[
                "pomme", "poire", "cerise", "fraise", "banane", "orange", "citron", "abricot", "pêche", "prune",
                "raisin", "melon", "kiwi", "ananas", "mangue", "carotte", "fenouil", "navet", "radis", "tomate",
                "courgette", "aubergine", "haricot", "épinard", "chou", "salade", "oignon", "potiron",
                "betterave", "figue",
            ],
            Theme::Cuisine => &[
                "hachoir", "poêle", "casserole", "fourchette", "couteau", "cuillère", "assiette", "verre",
                "tasse", "louche", "fouet", "farine", "sucre", "beurre", "recette", "gâteau", "tarte", "soupe",
                "omelette", "crêpe", "sauce", "épice", "poivre", "levure", "plat", "moutarde", "vinaigre",
                "fromage", "tablier", "passoire",
            ],
            Theme::Nature => &[
                "arbre", "forêt", "rivière", "montagne", "colline", "vallée", "prairie", "fleur", "feuille",
                "branche", "racine", "rocher", "sable", "nuage", "pluie", "orage", "neige", "soleil", "étoile",
                "océan", "plage", "falaise", "source", "étang", "champ", "mousse", "fougère", "chêne", "sapin",
                "volcan",
            ],
            Theme::Sport => &[
                "football", "tennis", "rugby", "judo", "natation", "course", "vélo", "escalade", "boxe", "golf",
                "voile", "aviron", "escrime", "danse", "karaté", "handball", "basket", "ballon", "raquette",
                "arbitre", "équipe", "match", "stade", "médaille", "podium", "sprint", "marathon", "plongeon",
                "patinage", "surf",
            ],
            Theme::Metiers => &[
                "boulanger", "médecin", "pompier", "facteur", "plombier", "menuisier", "jardinier", "infirmier",
                "pilote", "avocat", "juge", "peintre", "maçon", "cuisinier", "serveur", "coiffeur", "fleuriste",
                "libraire", "berger", "pêcheur", "marin", "chanteur", "potier", "dentiste", "architecte",
                "boucher", "fermier", "policier", "caissier", "notaire",
            ],
            Theme::Maison => &[
                "salon", "cuisine", "chambre", "grenier", "cave", "garage", "jardin", "fenêtre", "porte",
                "escalier", "plafond", "toit", "couloir", "balcon", "armoire", "canapé", "fauteuil", "table",
                "chaise", "lampe", "tapis", "rideau", "miroir", "étagère", "oreiller", "coussin", "placard",
                "cheminée", "baignoire", "parquet",
            ],
            Theme::Voyage => &[
                "valise", "avion", "train", "bateau", "billet", "passeport", "carte", "boussole", "hôtel",
                "plage", "musée", "gare", "aéroport", "croisière", "randonnée", "camping", "tente", "guide",
                "souvenir", "frontière", "escale", "départ", "arrivée", "touriste", "photo", "paysage",
                "auberge", "visite", "route", "vacances",
            ],
            Theme::Musique => &[
                "piano", "guitare", "violon", "flûte", "batterie", "trompette", "harpe", "tambour", "saxophone",
                "accordéon", "orgue", "clarinette", "note", "gamme", "rythme", "mélodie", "cymbale", "chanson",
                "refrain", "concert", "orchestre", "chorale", "partition", "solfège", "tempo", "opéra",
                "disque", "micro", "basse", "xylophone",
            ],
            Theme::Ecole => &[
                "cahier", "crayon", "gomme", "règle", "stylo", "cartable", "trousse", "tableau", "craie",
                "classe", "élève", "maître", "devoir", "leçon", "cantine", "dictée", "calcul", "lecture",
                "compas", "ciseaux", "colle", "livre", "page", "examen", "bulletin", "cours", "histoire",
                "géographie", "récréation", "équerre",
            ],
            Theme::Developpement => &[
                "code", "fonction", "variable", "boucle", "tableau", "objet", "classe", "méthode", "interface",
                "bogue", "débogage", "test", "fichier", "branche", "commit", "fusion", "dépôt", "logiciel",
                "algorithme", "syntaxe", "requête", "serveur", "module", "paquet", "octet", "chaîne", "entier",
                "pointeur", "pile", "récursion",
            ],
            Theme::Devops => &[
                "conteneur", "docker", "pipeline", "livraison", "cluster", "nuage", "image", "volume", "script",
                "automate", "alerte", "journal", "métrique", "sauvegarde", "réplique", "charge", "version",
                "kubernetes", "ansible", "terraform", "registre", "secret", "nœud", "astreinte", "incident",
                "migration", "production", "recette", "machine", "virtuelle",
            ],
            Theme::Reseaux => &[
                "routeur", "pont", "adresse", "paquet", "trame", "câble", "fibre", "wifi", "antenne", "serveur",
                "client", "protocole", "port", "masque", "passerelle", "pare-feu", "domaine", "latence", "débit",
                "bande", "signal", "modem", "ethernet", "proxy", "tunnel", "socket", "réseau", "nœud", "routage",
                "hôte",
            ],
            Theme::Ia => &[
                "neurone", "réseau", "modèle", "données", "corpus", "agent", "jeton", "invite", "vecteur",
                "poids", "biais", "gradient", "couche", "entrée", "sortie", "robot", "logique", "inférence",
                "algorithme", "calcul", "prédiction", "contexte", "mémoire", "langage", "assistant", "synapse",
                "tenseur", "perceptron", "attention", "erreur",
            ],
        }
    }
}

/// Mot tel qu'il se cherche dans la grille : majuscules sans accents, lettres seules.
pub(crate) fn grid_form(word: &str) -> String {
    let mut s = String::with_capacity(word.len());
    for c in word.chars().flat_map(char::to_lowercase) {
        let t = match c {
            'à' | 'â' | 'ä' | 'á' => "A",
            'é' | 'è' | 'ê' | 'ë' => "E",
            'î' | 'ï' | 'í' => "I",
            'ô' | 'ö' | 'ó' => "O",
            'ù' | 'û' | 'ü' | 'ú' => "U",
            'ÿ' => "Y",
            'ç' => "C",
            'ñ' => "N",
            'œ' => "OE",
            'æ' => "AE",
            c if c.is_ascii_lowercase() => {
                s.push(c.to_ascii_uppercase());
                continue;
            }
            _ => continue, // espaces, traits d'union, apostrophes
        };
        s.push_str(t);
    }
    s
}

/// (côté de la grille, nombre de mots, directions autorisées en (dx, dy))
fn settings(difficulty: Difficulty) -> (usize, usize, &'static [(i64, i64)]) {
    match difficulty {
        Difficulty::Easy => (10, 8, &[(1, 0), (0, 1)]),
        Difficulty::Medium => (12, 12, &[(1, 0), (0, 1), (1, 1), (1, -1)]),
        Difficulty::Hard => (14, 16, &[(1, 0), (0, 1), (1, 1), (1, -1), (-1, 0), (0, -1), (-1, -1), (-1, 1)]),
    }
}

fn hint(difficulty: Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Easy => "De gauche à droite et de haut en bas",
        Difficulty::Medium => "Horizontal, vertical et en diagonale",
        Difficulty::Hard => "Dans tous les sens, même à l'envers",
    }
}

pub struct Puzzle {
    pub size: usize,
    pub letters: Vec<u8>,
    /// Cases occupées par un mot caché, pour la solution.
    pub used: Vec<bool>,
    /// Mots placés, dans l'ordre alphabétique.
    pub words: Vec<String>,
}

/// Choisit les mots : forme de grille, longueur 3 à `size`, sans doublon ni mot contenu dans un autre.
fn pick_words(candidates: &[String], count: usize, size: usize, rng: &mut StdRng) -> Vec<String> {
    let mut pool: Vec<String> = candidates.iter().map(|w| grid_form(w)).collect();
    pool.shuffle(rng);
    let mut picked: Vec<String> = Vec::new();
    for w in pool {
        if picked.len() == count {
            break;
        }
        if (3..=size).contains(&w.len()) && !picked.iter().any(|p| p.contains(&w) || w.contains(p.as_str())) {
            picked.push(w);
        }
    }
    picked
}

pub fn generate(seed: u64, difficulty: Difficulty, candidates: &[String], count: usize) -> Puzzle {
    let mut rng = StdRng::seed_from_u64(seed);
    let (size, _, directions) = settings(difficulty);
    // La grille s'agrandit pour un mot plus long que son côté, jusqu'à 14 cases.
    let longest = candidates.iter().map(|w| grid_form(w).len()).filter(|&l| l <= MAX_SIZE).max().unwrap_or(0);
    let size = size.max(longest).min(MAX_SIZE);
    let mut words = pick_words(candidates, count, size, &mut rng);
    // Les plus longs d'abord : ce sont les plus difficiles à caser.
    words.sort_by_key(|w| std::cmp::Reverse(w.len()));

    let mut letters = vec![0u8; size * size];
    let mut used = vec![false; size * size];
    let mut placed = Vec::new();
    let n = size as i64;
    for word in words {
        let bytes = word.as_bytes();
        let len = bytes.len() as i64;
        for _ in 0..300 {
            let &(dx, dy) = directions.choose(&mut rng).expect("au moins une direction");
            let (x0, y0) = (rng.random_range(0..n), rng.random_range(0..n));
            let (x1, y1) = (x0 + dx * (len - 1), y0 + dy * (len - 1));
            if !(0..n).contains(&x1) || !(0..n).contains(&y1) {
                continue;
            }
            let cells: Vec<usize> = (0..len).map(|k| ((y0 + dy * k) * n + x0 + dx * k) as usize).collect();
            // Un mot peut croiser un autre, à condition de partager la lettre.
            if cells.iter().zip(bytes).all(|(&i, &b)| letters[i] == 0 || letters[i] == b) {
                for (&i, &b) in cells.iter().zip(bytes) {
                    letters[i] = b;
                    used[i] = true;
                }
                placed.push(word);
                break;
            }
        }
    }

    // Cases libres : lettres tirées parmi celles des mots, pour ne pas trahir les mots cachés.
    let pool: Vec<u8> = placed.iter().flat_map(|w| w.bytes()).collect();
    for l in letters.iter_mut().filter(|l| **l == 0) {
        *l = *pool.choose(&mut rng).unwrap_or(&b'E');
    }
    placed.sort();
    Puzzle { size, letters, used, words: placed }
}

/// Côté maximal : 14 lettres tiennent encore lisiblement sur 512 points.
const MAX_SIZE: usize = 14;
const BOTTOM_GAP: i64 = 10;
const FRAME: i64 = 4;

fn render(p: &Puzzle, solution: bool) -> image::GrayImage {
    let n = p.size as i64;
    let cell = ((512 - 2 * FRAME - 8) / n).min(40);
    let scale = (cell - 6) / draw::DIGIT_H;
    let size = cell * n;
    let (ox, oy) = ((512 - size) / 2, FRAME + 2);
    let mut img = draw::canvas((size + 2 * oy + BOTTOM_GAP) as u32);

    // Cadre autour de la grille.
    let (fx, fy, fs) = (ox - FRAME - 2, oy - FRAME - 2, size + 2 * (FRAME + 2));
    draw::fill_rect(&mut img, fx, fy, fs, FRAME);
    draw::fill_rect(&mut img, fx, fy + fs - FRAME, fs, FRAME);
    draw::fill_rect(&mut img, fx, fy, FRAME, fs);
    draw::fill_rect(&mut img, fx + fs - FRAME, fy, FRAME, fs);

    for (i, &c) in p.letters.iter().enumerate() {
        let (x, y) = (ox + (i as i64 % n) * cell, oy + (i as i64 / n) * cell);
        let ink = if solution && p.used[i] {
            draw::fill_rect(&mut img, x + 1, y + 1, cell - 2, cell - 2);
            draw::WHITE
        } else {
            draw::BLACK
        };
        let (lx, ly) = (x + (cell - draw::DIGIT_W * scale) / 2, y + (cell - draw::DIGIT_H * scale) / 2);
        draw::letter(&mut img, c, lx, ly, scale, ink);
    }
    img
}

pub fn build(
    difficulty: Difficulty,
    theme: Option<Theme>,
    words: &[String],
    seed: Option<u64>,
    solution: bool,
) -> Doc {
    let seed = seed.unwrap_or_else(|| rand::rng().random_range(1..100_000));
    let (_, count, _) = settings(difficulty);
    // Thème tiré de la graine, sans consommer d'aléa : préciser le thème ne change pas la grille.
    let theme = theme.unwrap_or(THEMES[(seed % THEMES.len() as u64) as usize]);
    let (candidates, count, subject) = if words.is_empty() {
        (theme.words().iter().map(|w| w.to_string()).collect(), count, Some(theme.label()))
    } else {
        (words.to_vec(), words.len(), None)
    };
    let puzzle = generate(seed, difficulty, &candidates, count);

    let mut doc = Doc::new();
    // La difficulté dans le bandeau, comme les autres jeux : il la faut pour imprimer la solution.
    let title = if solution { "Solution des mots mêlés" } else { "Mots mêlés" };
    doc.header(&format!("{title} · {}", difficulty.label()));
    if let Some(subject) = subject {
        doc.text(&format!("Thème : {subject}"), Style::default().small().center());
    }
    doc.text(hint(difficulty), Style::default().small().center());
    doc.feed(1);
    doc.image(render(&puzzle, solution));

    // Liste des mots en trois colonnes de 14 caractères (42 au total).
    for row in puzzle.words.chunks(3) {
        let line: String = row.iter().map(|w| format!("{w:^14}")).collect();
        doc.line(&line, Style::default());
    }
    doc.text(&format!("n° {seed}"), Style::default().small().align(Align::Right));
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme_words(theme: Theme) -> Vec<String> {
        theme.words().iter().map(|w| w.to_string()).collect()
    }

    /// Retrouve un mot dans la grille, dans n'importe quelle direction.
    fn find(p: &Puzzle, word: &str) -> bool {
        let n = p.size as i64;
        let w = word.as_bytes();
        let at = |x: i64, y: i64| p.letters[(y * n + x) as usize];
        (0..n).any(|y0| {
            (0..n).any(|x0| {
                [(1, 0), (0, 1), (1, 1), (1, -1), (-1, 0), (0, -1), (-1, -1), (-1, 1)].iter().any(|&(dx, dy)| {
                    w.iter().enumerate().all(|(k, &b)| {
                        let (x, y) = (x0 + dx * k as i64, y0 + dy * k as i64);
                        (0..n).contains(&x) && (0..n).contains(&y) && at(x, y) == b
                    })
                })
            })
        })
    }

    #[test]
    fn grid_form_strips_accents_and_punctuation() {
        assert_eq!(grid_form("écureuil"), "ECUREUIL");
        assert_eq!(grid_form("cœur"), "COEUR");
        assert_eq!(grid_form("arc-en-ciel"), "ARCENCIEL");
        assert_eq!(grid_form("Maître"), "MAITRE");
    }

    #[test]
    fn every_listed_word_is_in_the_grid() {
        for (seed, diff) in [(1, Difficulty::Easy), (42, Difficulty::Medium), (7, Difficulty::Hard)] {
            for theme in THEMES {
                let (_, count, _) = settings(diff);
                let p = generate(seed, diff, &theme_words(theme), count);
                assert!(p.words.len() >= count - 2, "{theme:?} {diff:?} : {} mots", p.words.len());
                assert!(p.letters.iter().all(u8::is_ascii_uppercase));
                for w in &p.words {
                    assert!(find(&p, w), "{w} introuvable");
                }
            }
        }
    }

    #[test]
    fn theme_words_fit_the_letter_font() {
        for theme in THEMES {
            for w in theme.words() {
                let g = grid_form(w);
                assert!(g.len() >= 3 && g.bytes().all(|b| b.is_ascii_uppercase()), "{w}");
                assert!(g.len() <= settings(Difficulty::Easy).0, "{w} : trop long pour la grille facile");
            }
            let forms: Vec<String> = theme.words().iter().map(|w| grid_form(w)).collect();
            for a in &forms {
                for b in &forms {
                    assert!(a == b || !a.contains(b.as_str()), "{theme:?} : {b} est dans {a}");
                }
            }
        }
    }

    #[test]
    fn grid_grows_for_long_custom_words() {
        let words = vec!["anniversaire".to_owned(), "gâteau".to_owned()];
        let p = generate(5, Difficulty::Easy, &words, 2);
        assert_eq!(p.size, 12);
        assert_eq!(p.words, ["ANNIVERSAIRE", "GATEAU"]);
    }

    #[test]
    fn same_seed_same_grid() {
        let words = theme_words(Theme::Nature);
        let a = generate(1234, Difficulty::Medium, &words, 12);
        let b = generate(1234, Difficulty::Medium, &words, 12);
        assert_eq!(a.letters, b.letters);
        assert_eq!(a.words, b.words);
    }
}
