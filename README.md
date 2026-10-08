# Printr · borne FabLab

Des jeux imprimés à la demande sur une imprimante à tickets **Epson TM-T88V**, pour la borne
installée à côté de la machine à café du FabLab : sudoku, mots mêlés, voie ferrée, logimage,
énigmes… On choisit son jeu, le ticket sort en quelques secondes.

Ce dépôt est un fork de [Printr](https://github.com/XNinety9/Printr), de X99, recentré sur la
borne : **tout fonctionne hors ligne**. Les blocs qui dépendent d'un service extérieur (météo,
actualités, horoscope Claude…) et l'interface web ont été retirés. Les nouveaux blocs de jeux
sont proposés au projet d'origine (voir [Contribuer](#contribuer)).

## Démarrage rapide

```sh
cargo run --release -- --preview print examples/complet.json   # aperçu dans le terminal, sans imprimante
scripts/demo.sh                                                 # rendu PNG dans l'émulateur
cargo run --release -- --device /dev/usb/lp0 print examples/matin.json
```

## Utilisation

```sh
printr print examples/matin.json    # ticket composé de blocs (voir plus bas)
cat ticket.json | printr print      # idem, depuis l'entrée standard
printr --preview print ticket.json  # aperçu dans le terminal, sans imprimer
printr -q print ticket.json         # silencieux, n'affiche que les erreurs
printr test                         # ticket de test (styles, accents, QR code)
printr text "Salut !"               # texte libre, puis coupe
echo "depuis stdin" | printr text   # lit l'entrée standard
printr text --no-cut "sans coupe"
printr image logo.png               # image réduite à 512 px, tramée (Floyd–Steinberg)
printr image --no-dither logo.png   # simple seuil noir/blanc, pour logos et dessins au trait
```

Destination, au choix :

| Option | Destination |
| --- | --- |
| `--device <chemin>` | imprimante USB (défaut `/dev/usb/lp0`, ou `$PRINTR_DEVICE`) |
| `--tcp hôte:port` | imprimante réseau, émulateur ou relais Windows |
| `--dump <fichier>` | écrit les octets ESC/POS bruts dans un fichier |

## Tickets JSON

Un ticket est une liste de blocs, imprimés dans l'ordre. Un bloc en échec imprime un message au
lieu de bloquer le ticket. Un paramètre inconnu est une erreur, pour repérer les fautes de frappe.

```json
{
  "cut": true,
  "spacing": 1,
  "blocks": [
    { "type": "title", "text": "Pause café" },
    { "type": "enigme" },
    { "type": "voie_ferree", "difficulty": "moyen" }
  ]
}
```

### Jeux

| Bloc | Paramètres (défaut) | Notes |
| --- | --- | --- |
| `sudoku` | `difficulty` (`facile`/`moyen`/`difficile`), `seed`, `solution` | Grille à solution unique |
| `word_search` (ou `mots_meles`, `mots_caches`, `mots_en_grille`) | `difficulty`, `theme`, `words`, `seed`, `solution` | Mots mêlés en français. `theme` : `animaux`, `fruits_legumes`, `cuisine`, `nature`, `sport`, `metiers`, `maison`, `voyage`, `musique`, `ecole`, `developpement`, `devops`, `reseaux`, `ia` (tiré du n° si absent) ; ou ses propres mots dans `words`. De 10×10 (vers la droite et le bas) à 14×14 (dans tous les sens) |
| `train_tracks` (ou `voie_ferree`, `rails`) | `difficulty`, `seed`, `solution` | Relier A (bord gauche) à B (bord bas) par une seule voie, d'après le nombre de cases de voie de chaque ligne et colonne. Grilles de 6×6, 8×8 et 10×10, à solution unique |
| `nonogram` (ou `logimage`) | `number` (1 à 10), `solution` | Logimage de 10×10, à solution unique, trouvable par déduction |
| `maze` | `width` (12), `height` (16), `seed` | Entrée en haut, sortie en bas |
| `riddle` (ou `enigme`) | `kind` (`devinette`/`charade`/`logique`/`calcul`), `number`, `answer` (`envers`/`lendemain`/`dessous`/`aucune`) | 125 énigmes, une par jour sans répétition ; réponse imprimée à l'envers par défaut |
| `mental_math` (ou `calcul_mental`) | `difficulty`, `count` (10), `seed` | Fiche d'opérations, résultats imprimés à l'envers |
| `anagram` (ou `mot_mystere`) | `theme` (comme `word_search`), `count` (3), `seed` | Lettres mélangées, réponses à l'envers |
| `petit_bac` | `players` (1), `letter`, `count` (6), `categories`, `seed` | Une lettre et des catégories, une feuille par joueur |
| `cipher` (ou `message_code`) | `message`, `cipher` (`cesar`/`morse`/`nombres`), `shift`, `answer` (true) | Message codé et sa grille de déchiffrement |
| `coloring` (ou `coloriage`) | `seed` | Mandala à colorier |

**Numéro de grille et solution.** Les grilles générées (sudoku, mots mêlés, voie ferrée,
labyrinthe…) portent un numéro en bas à droite : c'est leur graine. Pour imprimer la solution, on
reprend ce numéro dans `seed`, avec la même difficulté (et le même thème ou les mêmes mots pour
les mots mêlés), et on ajoute `"solution": true` :

```json
{ "type": "voie_ferree", "difficulty": "moyen", "seed": 4521, "solution": true }
```

### Mise en page et autres blocs

| Bloc | Paramètres (défaut) | Notes |
| --- | --- | --- |
| `title` | `text`, `size` (2) | Gras, centré, agrandi |
| `text` | `text`, `bold`, `underline`, `reverse`, `small`, `size` (1), `align` (`left`/`center`/`right`) | Coupé aux mots |
| `separator` | `style` (`-`) | Un caractère répété : `-`, `=`, `═`, `─`, `·`… |
| `date` | — | « Mercredi 7 octobre 2026 » |
| `feed` | `lines` (1) | Espace vertical |
| `image` | `path`, `dither` (true) | Fichier local réduit à 512 px (logo du FabLab…) |
| `qr` | `data`, `size` (6), `caption` | |
| `picto` | `shape` (`coeur`/`etoile`/`soleil`/`fleur`/`sourire`), `size` (`petit`/`moyen`/`grand`), `count` (1) | Petit dessin |
| `saint` | — | Calendrier local |
| `quote` | — | Citation du jour, liste locale |
| `holidays` | `zone` (`metropole`/`alsace-moselle`), `count` (1) | Prochains jours fériés, calcul local |
| `countdown` | `label`, `date` (`AAAA-MM-JJ`) | « J-79 avant : Noël » |
| `moon` | — | Phase, illumination, prochaines pleine et nouvelle lunes |
| `todo` | `items`, `title` (« À faire ») | Cases à cocher |
| `wifi` | `ssid`, `password`, `security` (`wpa`/`wep`/`none`), `hidden`, `show_password` (true) | QR code qui connecte au réseau |
| `bins` (ou `poubelles`) | `collections` (`name`, `days`, `every`, `from`), `when` (`veille`/`jour`), `always` | Rappel des jours de ramassage |
| `barnum` | `sign` ou `birth_date` (`AAAA-MM-JJ`), `sky` (false), `variant` | Horoscope hors ligne calculé par [Barnum](https://github.com/XNinety9/Barnum), à installer à part : `barnum` dans le PATH, ou la commande donnée dans `PRINTR_BARNUM` (par exemple `python3 /opt/barnum/main.py`) |

De temps en temps (une impression sur 20), la machine glisse dans le ticket un message étrange,
comme du bruit imprimé : c'est le **glitch**. `PRINTR_GLITCH` règle la fréquence
(`PRINTR_GLITCH=5` pour une sur cinq, `0` pour jamais) ; `{ "type": "glitch" }` en force un. Il
n'apparaît jamais dans les aperçus.

### Exemples

- [`examples/`](examples/) : tickets complets. `matin.json` (pause café), `complet.json` (tous
  les blocs), `extras.json`, et un ticket par jeu (`sudoku.json`, `mots-meles.json`,
  `voie-ferree.json`).
- [`docs/exemples/`](docs/exemples/) : un exemple par bloc, avec son rendu PNG, régénéré par
  `docs/exemples/generer.sh [bloc…]`.

## Borne tactile

L'interface de la borne vit dans [`kiosk/`](kiosk/), pour un écran tactile de 480 × 272 (carte
ESP32-S3 de 4,3″ visée) :

- [`kiosk/games.json`](kiosk/games.json) décrit la borne : son titre, son modèle de ticket par
  défaut, et la liste des jeux. Chaque jeu a un titre, une icône, une description, ses
  paramètres par défaut (`block`, au format des tickets ci-dessus) et les réglages proposés à
  l'écran (`options`). Ajouter un jeu ne demande qu'une entrée dans ce fichier.
- [`kiosk/templates/`](kiosk/templates/) contient les modèles de tickets : ce qui est imprimé
  autour du jeu choisi (voir ci-dessous).
- `kiosk/ui` (crate `borne-ui`) : les écrans (accueil, réglages, impression), dessinés avec
  [embedded-graphics](https://docs.rs/embedded-graphics), sans dépendance au matériel.
- `kiosk/sim` (crate `borne-sim`) : un simulateur qui joue des appuis, enregistre chaque écran en
  PNG dans `kiosk/captures/` et écrit les tickets produits.

```sh
cargo run -p borne-sim                                            # visite guidée en captures
cargo run -p borne-sim -- tap:299,97 tap:364,238 | cargo run -- --preview print
```

Un jeu dans `games.json` :

```json
{
  "title": "Sudoku",
  "icon": "sudoku",
  "description": "Une grille à solution unique.",
  "block": { "type": "sudoku", "difficulty": "moyen" },
  "options": [
    { "label": "Niveau", "key": "difficulty", "choices": [
      { "label": "Facile", "value": "facile" },
      { "label": "Moyen", "value": "moyen" },
      { "label": "Difficile", "value": "difficile" }
    ] }
  ]
}
```

- `icon` : une icône intégrée (`sudoku`, `loupe`, `rails`, `labyrinthe`, `coeur`, `question`,
  `calcul`, `lettre`, `crayon`, `enveloppe`, `fleur`, `haltere`, `etoile`, `cafe`), ou un dessin en lignes
  de `#` et de `.` (32 × 32 au plus).
- `options` : chaque choix donne la valeur du paramètre `key` ; `null` retire le paramètre (Printr
  choisit alors au hasard). Sans `key`, chaque valeur est un objet fusionné dans le bloc, par
  exemple `{ "width": 8, "height": 10 }` pour un petit labyrinthe.
- Jusqu'à quatre choix, ils s'affichent côte à côte ; au-delà, avec des flèches.
- `solution` : ajoute un bouton « Solution » en bas à gauche de l'écran du jeu. On y tape le
  numéro imprimé sur la grille, et la borne imprime la solution avec les réglages choisis à
  l'écran (qui doivent être ceux du ticket). `key` est le paramètre qui reçoit le numéro (`seed`
  pour les grilles générées, `number` pour les logimages), `max` le plus grand numéro accepté, et
  `set` ce qui est ajouté au bloc (`{"solution": true}` par défaut) :
  `"solution": { "key": "number", "max": 10 }`.

### Packs de jeux

Une entrée avec `pack` à la place de `block` imprime plusieurs jeux sur le même ticket, chacun avec
ses réglages par défaut. Les jeux y sont désignés par leur `id` :

```json
{
  "title": "Pack pause",
  "icon": "cafe",
  "description": "Cinq jeux tirés au hasard, pour toute la pause.",
  "pack": { "pick": "hasard", "count": 5 },
  "template": "pack"
}
```

| `pick` | Jeux du pack |
| --- | --- |
| `tous` (défaut) | Tous ceux de `games`, dans l'ordre |
| `hasard` | `count` jeux (5 par défaut) tirés au hasard, différents à chaque ticket |
| `populaires` | Les `count` jeux les plus imprimés sur la borne (à égalité, l'ordre de `games.json`) |

`games` limite le choix à une liste d'`id` (`"games": ["sudoku", "mots-meles", "voie-ferree"]`) ;
vide, tous les jeux de la borne sont candidats. `exclude` en écarte certains
(`"exclude": ["petit-bac"]`). Un pack s'affiche en tuile orangée et liste son
contenu à l'écran. La borne compte les impressions de chaque jeu ; l'appareil peut sauvegarder ce
décompte (`App::popularity`) et le reprendre au démarrage (`App::set_popularity`).

### Modèles de tickets

Un modèle est un ticket au format habituel (comme ceux de [`examples/`](examples/)), où le bloc
`{ "type": "jeu" }` marque la place du jeu choisi, avec ses réglages. Par exemple
[`kiosk/templates/defaut.json`](kiosk/templates/defaut.json) :

```json
{
  "cut": true,
  "spacing": 1,
  "blocks": [
    { "type": "title", "text": "FabLab" },
    { "type": "date" },
    { "type": "jeu" },
    { "type": "separator", "style": "─" },
    { "type": "text", "text": "Bonne pause !", "small": true, "align": "center" }
  ]
}
```

- Le modèle utilisé par défaut est donné par `"template"` dans `games.json` (`defaut` si absent) ;
  un jeu peut en choisir un autre avec son propre `"template"`, par exemple
  `"template": "coloriage"` pour `kiosk/templates/coloriage.json`.
- Le nom d'un modèle est celui de son fichier, sans `.json`. Chaque modèle contient un et un
  seul bloc `jeu`.
- Pour un pack, le bloc `jeu` reçoit tous ses jeux, à la suite. `entre` donne les blocs à
  imprimer entre deux jeux, par exemple dans
  [`kiosk/templates/pack.json`](kiosk/templates/pack.json) :
  `{ "type": "jeu", "entre": [{ "type": "separator", "style": "═" }] }`.
- Pour voir le ticket complet d'un jeu : `cargo run -p borne-sim -- tap:299,97 tap:364,238 |
  cargo run -- --preview print`.

Une impression sur 100 000 (0,001 %) depuis la borne est précédée d'un
[glitch](#mise-en-page-et-autres-blocs), seul sur son propre ticket, coupé, avant le ticket
demandé (`App::set_glitch_odds` pour changer la fréquence, `0` pour jamais). La borne demande
alors deux impressions à la suite ; le simulateur écrit un fichier et une ligne par ticket.

`cargo test` vérifie que chaque ticket que la borne peut produire (chaque jeu, chaque réglage,
dans son modèle) est valide et s'imprime sans erreur.

## Tester sans imprimante (émulateur)

[emupos](https://pypi.org/project/emupos/) simule l'imprimante et rend chaque ticket en PNG + texte.
Le dossier `emulator/` contient un profil TM-T88V (512 points, 180 dpi).

```sh
scripts/demo.sh                          # examples/complet.json par défaut
scripts/demo.sh examples/voie-ferree.json
# → affiche le chemin du rendu, emulator/receipts/<id>.png (et .txt)
```

Le script lance l'émulateur, imprime le ticket, attend le rendu puis arrête l'émulateur.
À la main, dans deux terminaux :

```sh
cd emulator && uvx emupos run                # terminal 1
cargo run -- --tcp 127.0.0.1:9100 test       # terminal 2
```

## Développer sous Windows (dev container)

Le dossier `.devcontainer/` fournit un environnement Linux avec Rust et uv : dans VS Code
(extension Dev Containers, Docker Desktop lancé), **Reopen in Container**. Tout se fait ensuite
dans le terminal du conteneur :

```sh
cargo run -- --preview print ticket.json                        # aperçu dans le terminal
scripts/demo.sh ticket.json                                      # rendu PNG via l'émulateur
cargo run -- --tcp host.docker.internal:9101 print ticket.json   # vraie imprimante, via le relais
```

### Relais PowerShell vers l'imprimante USB

L'imprimante USB reste branchée sur Windows. Le script
[`scripts/windows-relay.ps1`](scripts/windows-relay.ps1) écoute sur `127.0.0.1:9101` côté Windows
et transmet chaque connexion reçue à la file d'impression, en brut (RAW, sans le rendu du pilote).
Un ticket = une connexion : il est imprimé à la fermeture de celle-ci.

Dans un terminal PowerShell **Windows**, à la racine du dépôt, et le laisser ouvert :

```powershell
powershell -ExecutionPolicy Bypass -File scripts\windows-relay.ps1
# Relais 127.0.0.1:9101 -> 'EPSON TM-T88V Receipt' (Ctrl+C pour arrêter)
```

Options : `-Printer "nom"` si la file Windows porte un autre nom (liste : `Get-Printer | Select Name`),
`-Port 9102` pour un autre port.

En cas de souci :

- **`impossible de se connecter à host.docker.internal:9101`** : le relais ne tourne pas (ou sur un
  autre port). Vérifier côté Windows avec `Test-NetConnection 127.0.0.1 -Port 9101`.
- **Le script s'arrête au démarrage** : la file `-Printer` n'existe pas ; prendre le nom exact
  donné par `Get-Printer`.
- **Connexion acceptée mais rien ne sort** : vérifier que l'imprimante est allumée et que la file
  Windows n'est pas en pause ou en erreur.

## Raspberry Pi

Compilation avec [`cross`](https://github.com/cross-rs/cross) (nécessite Docker) :

```sh
cross build --release --target aarch64-unknown-linux-gnu    # Pi 3 / Zero 2 W, OS 64 bits
cross build --release --target arm-unknown-linux-gnueabihf  # Pi Zero W (ARMv6)
scp target/aarch64-unknown-linux-gnu/release/printr pi@raspberrypi:
```

L'imprimante USB est exposée par le module noyau `usblp` en `/dev/usb/lp0`. Pour y accéder sans
`sudo` :

```sh
sudo cp deploy/70-tm-t88v.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
sudo usermod -aG lp $USER   # puis se reconnecter
```

La règle crée aussi le lien stable `/dev/tm88` : `printr --device /dev/tm88 test`.

## Contribuer

Les nouveaux blocs de jeux sont proposés au projet d'origine,
[XNinety9/Printr](https://github.com/XNinety9/Printr), en suivant son guide
[`CONTRIBUTING.md`](https://github.com/XNinety9/Printr/blob/master/CONTRIBUTING.md) : une branche partant de son `master`, puis une pull request.
La structure des blocs (`src/blocks/`, l'enum `Block`, `draw.rs`, `doc.rs`) est gardée identique
à la sienne pour qu'un bloc passe facilement de l'un à l'autre.
