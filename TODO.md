# À faire

Ce qui reste pour la borne de jeux du FabLab. Le logiciel (interface tactile, simulateur, jeux,
modèles de tickets) fonctionne ; il reste surtout la carte ESP32.

## Carte ESP32

- [ ] **Identifier la carte exacte** (lien ou modèle imprimé sur le circuit). Deux familles
  correspondent à « ESP32-S3 4,3″ 480 × 272, 8 Mo de PSRAM, 4 Mo de flash, tactile capacitif »,
  avec des écrans pilotés différemment :
  - Guition JC4827W543 : contrôleur NV3041A en QSPI ;
  - Sunton ESP32-4827S043 : écran RGB parallèle.

  Le tactile est très probablement un GT911 (I2C) dans les deux cas.
- [ ] **Programme de la carte** (Rust, ESP-IDF) autour de `borne-ui` :
  - afficher le `Framebuffer` à l'écran ;
  - transformer chaque appui en `Event::Tap`, et envoyer régulièrement des `Event::Tick` ;
  - imprimer les tickets de `Effect::Print` l'un après l'autre, puis appeler `print_finished`.
- [ ] **Graine du tirage** au démarrage : `app.seed()` avec le générateur matériel de l'ESP32.
- [ ] **Popularité** : sauvegarder `app.popularity()` en flash après chaque impression, et la
  reprendre avec `app.set_popularity()` au démarrage.
- [ ] **Imprimer depuis l'ESP32** : la borne produit des tickets JSON, mais les jeux sont générés
  par Printr, qui ne tourne pas encore sur ESP32. Adapter le moteur (blocs, mise en page,
  ESC/POS) avec la version ESP-IDF de Rust, qui gère la bibliothèque standard. C'est le plus gros
  morceau, à vérifier sur la carte.
- [ ] **Liaison avec l'imprimante** : USB ou RS232, selon l'imprimante et la carte. En RS232, un
  adaptateur (MAX3232, ~2 €).
- [ ] **Glitch de Printr** : le désactiver sur la borne (équivalent de `PRINTR_GLITCH=0`), pour ne
  garder que le ticket glitch à 0,001 % de l'interface.

## Décisions à prendre

- [ ] **Énigme** : c'est une « énigme du jour », la même pour tout le monde toute la journée.
  La garder ainsi, ou en tirer une au hasard à chaque impression ?
- [ ] **Petit bac** : à retravailler ; en attendant, il est écarté des packs (`exclude`).
- [ ] **Calcul mental** : retiré de la borne, mais le bloc `calcul_mental` est toujours dans le
  code. Le supprimer aussi ?
- [ ] **Icône « haltere »** : plus utilisée depuis le retrait du défi sportif. La garder pour un
  futur jeu ?
- [x] **Longueur des packs** : 4 jeux par pack, sans coloriage ni logimage (environ 50 cm).

## Vérifications

- [x] Imprimer sur la vraie imprimante le nouveau logimage (centré, grandes cases), dans un pack.
- [ ] Imprimer sur la vraie imprimante un ticket de solution demandé depuis la borne, et vérifier
  qu'il correspond bien à la grille d'origine.

## Documentation

- [ ] Régénérer l'animation du README après chaque changement de l'interface :
  `cargo run -p borne-sim -- --seed 7 --glitch 0 --gif docs/borne.gif`.

## Suivi du projet d'origine

- [ ] La PR « Bloc voie ferrée » (XNinety9/Printr) attend d'être fusionnée.
- [ ] Reprendre les nouveaux commits de X99 (par exemple : `--dump` et `--tcp` l'emportent sur
  `PRINTR_DEVICE`).
- [ ] Les nouveaux jeux pensés pour la borne sont à proposer à X99 en PR : une branche partant de
  son `master`, en suivant son guide de contribution.
