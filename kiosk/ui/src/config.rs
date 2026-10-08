//! Description de la borne, lue dans `games.json` : titre, jeux proposés, paramètres par défaut
//! et options réglables à l'écran ; et modèles de tickets, lus dans `templates/`.
//!
//! La borne ne connaît pas les jeux : chaque jeu est un bloc de ticket Printr (`block`), que les
//! options viennent compléter, puis qui prend la place du bloc `{"type": "jeu"}` de son modèle.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::icons;

/// Type du bloc qui, dans un modèle, marque la place du jeu.
pub const PLACEHOLDER: &str = "jeu";
/// Modèle utilisé par défaut ; s'il n'existe pas, le ticket ne contient que le jeu.
pub const DEFAULT_TEMPLATE: &str = "defaut";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Titre affiché en haut de l'écran d'accueil.
    pub title: String,
    /// Modèle de ticket des jeux qui n'en précisent pas.
    #[serde(default = "default_template")]
    pub template: String,
    pub games: Vec<Game>,
    /// Réglages du mode maintenance.
    #[serde(default)]
    pub maintenance: Maintenance,
    /// Modèles de tickets, par nom (le nom du fichier sans `.json`).
    #[serde(skip)]
    pub templates: BTreeMap<String, Value>,
}

/// Mode maintenance : ouvert par un appui de 6 s sur le titre de l'accueil.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Maintenance {
    /// Réseau que la borne rejoint tant que le mode maintenance est ouvert.
    #[serde(default)]
    pub wifi: WifiNetwork,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WifiNetwork {
    pub ssid: String,
    pub password: String,
}

fn default_template() -> String {
    DEFAULT_TEMPLATE.to_owned()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Game {
    /// Identifiant, pour désigner le jeu dans un pack (`"sudoku"`, `"mots-meles"`…).
    #[serde(default)]
    pub id: Option<String>,
    pub title: String,
    pub icon: Icon,
    /// Une phrase sous l'icône, sur l'écran du jeu.
    #[serde(default)]
    pub description: Option<String>,
    /// Modèle de ticket propre à ce jeu, à la place du modèle par défaut.
    #[serde(default)]
    pub template: Option<String>,
    /// Bloc Printr avec ses paramètres par défaut, par exemple `{"type": "sudoku", "difficulty": "moyen"}`.
    #[serde(default)]
    pub block: Option<Map<String, Value>>,
    /// À la place de `block` : un pack de plusieurs jeux sur le même ticket.
    #[serde(default)]
    pub pack: Option<Pack>,
    /// Réglages proposés à l'écran.
    #[serde(default)]
    pub options: Vec<GameOption>,
    /// Si le jeu sait imprimer sa solution d'après le numéro de la grille : bouton « Solution ».
    #[serde(default)]
    pub solution: Option<Solution>,
}

/// Impression d'une solution : le numéro saisi va dans le paramètre `key`, et `set` est ajouté
/// au bloc (par défaut `{"solution": true}`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Solution {
    pub key: String,
    /// Plus grand numéro accepté (`10` pour les logimages, par exemple).
    #[serde(default)]
    pub max: Option<u64>,
    #[serde(default = "solution_flag")]
    pub set: Map<String, Value>,
}

fn solution_flag() -> Map<String, Value> {
    Map::from_iter([("solution".to_owned(), Value::Bool(true))])
}

impl Solution {
    /// Le numéro est-il acceptable ?
    pub fn accepts(&self, number: u64) -> bool {
        number >= 1 && self.max.is_none_or(|max| number <= max)
    }
}

impl Game {
    /// Clé du jeu : son identifiant, sinon son titre (pour les statistiques de popularité).
    pub fn key(&self) -> &str {
        self.id.as_deref().unwrap_or(&self.title)
    }

    pub fn is_pack(&self) -> bool {
        self.pack.is_some()
    }
}

/// Un pack : plusieurs jeux imprimés à la suite, avec leurs réglages par défaut.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pack {
    /// Comment choisir les jeux du pack.
    #[serde(default)]
    pub pick: Pick,
    /// Nombre de jeux, pour `hasard` et `populaires`.
    #[serde(default = "five")]
    pub count: usize,
    /// Jeux parmi lesquels choisir (leurs `id`) ; tous les jeux de la borne si vide.
    #[serde(default)]
    pub games: Vec<String>,
    /// Jeux à écarter du pack (leurs `id`).
    #[serde(default)]
    pub exclude: Vec<String>,
}

fn five() -> usize {
    5
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pick {
    /// Tous les jeux de la liste, dans l'ordre.
    #[default]
    Tous,
    /// `count` jeux tirés au hasard, différents à chaque ticket.
    Hasard,
    /// Les `count` jeux les plus imprimés sur la borne.
    Populaires,
}

/// Icône d'un jeu : le nom d'une icône intégrée, ou un dessin en lignes de `#` et de `.`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Icon {
    Named(String),
    Pixels(Vec<String>),
}

impl Icon {
    fn row(&self, y: usize) -> Option<&str> {
        match self {
            Icon::Named(name) => icons::builtin(name).unwrap_or(icons::FALLBACK).get(y).copied(),
            Icon::Pixels(rows) => rows.get(y).map(String::as_str),
        }
    }

    /// Largeur et hauteur du dessin, en points.
    pub fn size(&self) -> (usize, usize) {
        let height = match self {
            Icon::Named(name) => icons::builtin(name).unwrap_or(icons::FALLBACK).len(),
            Icon::Pixels(rows) => rows.len(),
        };
        (self.row(0).map_or(0, str::len), height)
    }

    /// Le point (x, y) est-il allumé ?
    pub fn is_on(&self, x: usize, y: usize) -> bool {
        self.row(y).is_some_and(|row| row.as_bytes().get(x) == Some(&b'#'))
    }
}

/// Un réglage : soit la valeur d'un paramètre du bloc (`key`), soit, sans `key`, un groupe de
/// paramètres fusionné dans le bloc (chaque `value` est alors un objet JSON).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameOption {
    pub label: String,
    #[serde(default)]
    pub key: Option<String>,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    pub label: String,
    /// `null` retire le paramètre du bloc (Printr choisit alors au hasard).
    pub value: Value,
}

impl Config {
    /// Lit `games.json` et les modèles de `templates/*.json` dans le dossier `dir`.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let read = |path: &Path| std::fs::read_to_string(path).map_err(|e| format!("{} : {e}", path.display()));
        let games = read(&dir.join("games.json"))?;
        let mut templates = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir.join("templates")) {
            for path in entries.filter_map(|e| e.ok().map(|e| e.path())) {
                if path.extension().is_some_and(|ext| ext == "json") {
                    let name = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
                    templates.push((name, read(&path)?));
                }
            }
        }
        Self::from_parts(&games, templates)
    }

    /// Construit la configuration à partir de textes déjà chargés (fichiers intégrés au programme).
    pub fn from_parts(games: &str, templates: impl IntoIterator<Item = (String, String)>) -> Result<Self, String> {
        let mut config: Config = serde_json::from_str(games).map_err(|e| format!("games.json invalide : {e}"))?;
        for (name, json) in templates {
            let template = serde_json::from_str(&json).map_err(|e| format!("modèle « {name} » invalide : {e}"))?;
            config.templates.insert(name, template);
        }
        config.validate()?;
        Ok(config)
    }

    /// `games.json` seul, sans modèles de tickets.
    pub fn from_json(json: &str) -> Result<Self, String> {
        Self::from_parts(json, [])
    }

    /// Jeux parmi lesquels un pack choisit (indices dans `games`).
    pub fn pack_pool(&self, entry: usize) -> Vec<usize> {
        let Some(pack) = &self.games[entry].pack else { return Vec::new() };
        let pool: Vec<usize> = if pack.games.is_empty() {
            (0..self.games.len()).filter(|&g| !self.games[g].is_pack()).collect()
        } else {
            pack.games.iter().filter_map(|id| self.games.iter().position(|g| g.id.as_ref() == Some(id))).collect()
        };
        pool.into_iter().filter(|&g| self.games[g].id.as_ref().is_none_or(|id| !pack.exclude.contains(id))).collect()
    }

    /// Modèle de ticket d'un jeu.
    fn template_of(&self, game: &Game) -> Value {
        let name = game.template.as_deref().unwrap_or(&self.template);
        self.templates
            .get(name)
            .cloned()
            .unwrap_or_else(|| serde_json::json!({ "blocks": [{ "type": PLACEHOLDER }] }))
    }

    fn validate(&self) -> Result<(), String> {
        if self.games.is_empty() {
            return Err("games.json : aucun jeu".into());
        }
        for (name, template) in &self.templates {
            let blocks = template.get("blocks").and_then(Value::as_array);
            let places: Vec<&Value> = blocks.into_iter().flatten().filter(|b| is_placeholder(b)).collect();
            if places.len() != 1 {
                return Err(format!("modèle « {name} » : il faut un et un seul bloc {{\"type\": \"{PLACEHOLDER}\"}} dans `blocks`"));
            }
            let place = places[0].as_object().expect("bloc objet");
            if place.keys().any(|k| k != "type" && k != "entre") || place.get("entre").is_some_and(|e| !e.is_array()) {
                return Err(format!("modèle « {name} » : le bloc `{PLACEHOLDER}` n'accepte que `entre` (une liste de blocs)"));
            }
        }
        let ids: Vec<&String> = self.games.iter().filter_map(|g| g.id.as_ref()).collect();
        if let Some(id) = ids.iter().enumerate().find_map(|(i, id)| ids[..i].contains(id).then_some(id)) {
            return Err(format!("identifiant « {id} » utilisé deux fois"));
        }
        let names = std::iter::once(&self.template).chain(self.games.iter().filter_map(|g| g.template.as_ref()));
        for name in names {
            if name != DEFAULT_TEMPLATE && !self.templates.contains_key(name) {
                return Err(format!("modèle « {name} » introuvable (fichier templates/{name}.json)"));
            }
        }
        for game in &self.games {
            let name = &game.title;
            match (&game.block, &game.pack) {
                (Some(block), None) if block.get("type").is_some_and(Value::is_string) => {}
                (Some(_), None) => return Err(format!("« {name} » : `block` doit avoir un `type`")),
                (None, Some(pack)) => {
                    if game.solution.is_some() {
                        return Err(format!("« {name} » : un pack n'a pas de solution"));
                    }
                    if !game.options.is_empty() {
                        return Err(format!("« {name} » : un pack n'a pas de réglages (`options`)"));
                    }
                    if pack.count == 0 {
                        return Err(format!("« {name} » : `count` doit valoir au moins 1"));
                    }
                    if self.pack_pool(self.games.iter().position(|g| std::ptr::eq(g, game)).expect("jeu de la liste")).is_empty() {
                        return Err(format!("« {name} » : aucun jeu à tirer"));
                    }
                    for id in pack.games.iter().chain(&pack.exclude) {
                        match self.games.iter().find(|g| g.id.as_ref() == Some(id)) {
                            None => return Err(format!("« {name} » : jeu « {id} » introuvable (aucun jeu n'a cet `id`)")),
                            Some(g) if g.is_pack() => return Err(format!("« {name} » : « {id} » est un pack")),
                            Some(_) => {}
                        }
                    }
                }
                _ => return Err(format!("« {name} » : il faut soit `block`, soit `pack`")),
            }
            match &game.icon {
                Icon::Named(icon) if icons::builtin(icon).is_none() => {
                    return Err(format!("« {name} » : icône inconnue « {icon} » (connues : {})", icons::names().join(", ")));
                }
                Icon::Pixels(rows) => icons::check(rows).map_err(|e| format!("« {name} » : icône : {e}"))?,
                Icon::Named(_) => {}
            }
            for option in &game.options {
                if option.choices.is_empty() {
                    return Err(format!("« {name} » : l'option « {} » n'a aucun choix", option.label));
                }
                if option.key.is_none() && !option.choices.iter().all(|c| c.value.is_object()) {
                    return Err(format!("« {name} » : sans `key`, chaque choix de « {} » doit être un objet", option.label));
                }
            }
        }
        Ok(())
    }

    /// Choix sélectionnés par défaut : celui qui correspond au bloc, sinon le premier.
    pub fn default_selection(&self, game: usize) -> Vec<usize> {
        let game = &self.games[game];
        let Some(block) = &game.block else { return Vec::new() };
        game.options
            .iter()
            .map(|option| {
                option
                    .choices
                    .iter()
                    .position(|choice| match &option.key {
                        Some(key) => block.get(key).unwrap_or(&Value::Null) == &choice.value,
                        None => choice.value.as_object().is_some_and(|set| {
                            set.iter().all(|(k, v)| block.get(k).unwrap_or(&Value::Null) == v)
                        }),
                    })
                    .unwrap_or(0)
            })
            .collect()
    }

    /// Bloc d'un jeu, avec ses choix appliqués.
    fn game_block(&self, game: usize, selection: &[usize]) -> Map<String, Value> {
        let game = &self.games[game];
        let mut block = game.block.clone().unwrap_or_default();
        for (option, &chosen) in game.options.iter().zip(selection) {
            let value = &option.choices[chosen].value;
            match (&option.key, value) {
                (Some(key), Value::Null) => {
                    block.remove(key);
                }
                (Some(key), value) => {
                    block.insert(key.clone(), value.clone());
                }
                (None, Value::Object(set)) => {
                    for (k, v) in set {
                        match v {
                            Value::Null => block.remove(k),
                            v => block.insert(k.clone(), v.clone()),
                        };
                    }
                }
                (None, _) => {}
            }
        }
        block
    }

    /// Ticket Printr complet pour un jeu et ses choix, dans son modèle.
    pub fn ticket(&self, game: usize, selection: &[usize]) -> Value {
        let block = self.game_block(game, selection);
        self.assemble(&self.games[game], vec![block])
    }

    /// Ticket de la solution d'une grille, d'après son numéro et les réglages choisis à l'écran.
    pub fn solution_ticket(&self, game: usize, selection: &[usize], number: u64) -> Value {
        let mut block = self.game_block(game, selection);
        if let Some(solution) = &self.games[game].solution {
            block.insert(solution.key.clone(), Value::from(number));
            block.extend(solution.set.clone());
        }
        self.assemble(&self.games[game], vec![block])
    }

    /// Ticket d'un pack : les jeux `chosen` (indices dans `games`), avec leurs réglages par défaut.
    pub fn pack_ticket(&self, entry: usize, chosen: &[usize]) -> Value {
        let blocks = chosen.iter().map(|&g| self.game_block(g, &self.default_selection(g))).collect();
        self.assemble(&self.games[entry], blocks)
    }

    /// Place les blocs des jeux dans le modèle, à la place du bloc `jeu`, séparés par ses blocs `entre`.
    fn assemble(&self, entry: &Game, games: Vec<Map<String, Value>>) -> Value {
        let mut ticket = self.template_of(entry);
        if let Some(blocks) = ticket.get_mut("blocks").and_then(Value::as_array_mut)
            && let Some(at) = blocks.iter().position(is_placeholder)
        {
            let place = blocks.remove(at);
            let between = place.get("entre").and_then(Value::as_array).cloned().unwrap_or_default();
            let mut filled = Vec::new();
            for (k, game) in games.into_iter().enumerate() {
                if k > 0 {
                    filled.extend(between.iter().cloned());
                }
                filled.push(Value::Object(game));
            }
            blocks.splice(at..at, filled);
        }
        ticket
    }
}

fn is_placeholder(block: &Value) -> bool {
    block.get("type").and_then(Value::as_str) == Some(PLACEHOLDER)
}
