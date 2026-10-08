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
- [ ] **Appui long** : envoyer le doigt posé et levé (`Event::Down`, `Event::Up`) au lieu de
  simples `Event::Tap`, pour ouvrir le mode maintenance.
- [ ] **Wi-Fi du mode maintenance** : tant que `app.wifi_wanted()` est vrai, allumer le Wi-Fi et
  chercher le réseau de `config().maintenance.wifi`, en réessayant sans fin ; donner l'état avec
  `app.set_wifi()` (recherche, puis force du signal et adresse) ; couper le Wi-Fi dès que le mode
  est quitté.
- [ ] **Horloge** : une RTC (celle de l'ESP32 ou un module externe, à choisir selon la dérive
  mesurée), lue chaque seconde et donnée à `app.set_clock()`.
- [ ] **Synchronisation NTP** : quand `app.ntp_wanted()` devient vrai (maintenance, Wi-Fi
  connecté), interroger un serveur NTP et mettre la RTC à l'heure (`app.set_ntp(Syncing)`, puis
  `Synced` ou `Failed`). Réessayer après un échec, par exemple toutes les 30 secondes. Une fois
  synchronisée, mesurer l'écart RTC − heure NTP en réinterrogeant le serveur régulièrement
  (toutes les 10 à 60 s, sans corriger l'horloge), et le redonner avec `Synced { offset_ms }`.
  L'estimer avec le compteur de l'ESP32 comparerait la RTC à son propre quartz, pas à l'heure
  réelle : si la RTC est celle de l'ESP32, la dérive resterait invisible.
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

## Mode maintenance : outils à venir

- [ ] **Mise à jour (OTA)** : envoyer un nouveau programme (et un nouveau `games.json`) par le
  Wi-Fi de maintenance.
- [ ] **Statistiques** : afficher les impressions par jeu (`app.popularity()`) et le papier
  consommé.
- [ ] **Envoi des statistiques** : les transmettre (format et destination à définir).
- [ ] **Sortie automatique** : faut-il quitter la maintenance après un long moment sans appui,
  pour qu'une borne oubliée en maintenance redevienne jouable ? Aujourd'hui, elle y reste.
- [ ] **Mot de passe du Wi-Fi** : il est dans `games.json`, donc public. Réserver ce réseau à la
  maintenance (partage de connexion de téléphone), ou sortir les identifiants du dépôt.

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
