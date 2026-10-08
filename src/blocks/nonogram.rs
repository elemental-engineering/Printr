//! Logimage (picross) : noircir les cases d'après les indices pour révéler un dessin.
//! Chaque dessin a une solution unique, trouvable par simple déduction ligne à ligne (vérifié
//! par les tests). Le numéro imprimé est celui du dessin : `solution: true` imprime le dessin.

use rand::RngExt;
use image::GrayImage;

use crate::doc::{Align, Doc, Style};
use crate::draw;

const N: usize = 10;

/// (nom, dessin de 10×10, `#` = case noire)
const PICTURES: &[(&str, [&str; N])] = &[
    ("cœur", [
        ".##....##.",
        "####..####",
        "##########",
        "##########",
        "##########",
        ".########.",
        "..######..",
        "...####...",
        "....##....",
        "..........",
    ]),
    ("sapin", [
        "....##....",
        "...####...",
        "..######..",
        "....##....",
        "..######..",
        ".########.",
        "...####...",
        "##########",
        "....##....",
        "...####...",
    ]),
    ("bateau", [
        "....#.....",
        "....##....",
        "....###...",
        "....####..",
        "....#####.",
        "....#.....",
        "##########",
        ".########.",
        "..######..",
        "~~~~~~~~~~",
    ]),
    ("sourire", [
        "..######..",
        ".########.",
        "##.####.##",
        "##.####.##",
        "##########",
        "#.######.#",
        "##.####.##",
        "###....###",
        ".########.",
        "..######..",
    ]),
    ("champignon", [
        "...####...",
        ".##.##.##.",
        "##########",
        "#.##..##.#",
        "##########",
        "...#..#...",
        "...#..#...",
        "...####...",
        "..........",
        "..........",
    ]),
    ("parapluie", [
        "....##....",
        "..######..",
        ".########.",
        "##########",
        "#.#.##.#.#",
        "....##....",
        "....##....",
        "....##....",
        ".#..##....",
        ".####.....",
    ]),
    ("maison", [
        "....##....",
        "...####...",
        "..######..",
        ".########.",
        "##########",
        ".#......#.",
        ".#.##.#.#.",
        ".#.##...#.",
        ".#.##...#.",
        ".########.",
    ]),
    ("chat", [
        "#...#.....",
        "##.##.....",
        "#####.....",
        "#.#.#.....",
        "#####....#",
        ".###.....#",
        "#####...#.",
        "######.##.",
        "#######...",
        "##.##.##..",
    ]),
    ("poisson", [
        "..........",
        "...####...",
        ".#######.#",
        "##.######.",
        "#########.",
        ".#######.#",
        "...####...",
        "....#.....",
        "..........",
        "..........",
    ]),
    ("tasse", [
        "...#.#....",
        "....#.#...",
        "...#.#....",
        "..........",
        "########..",
        "##########",
        "########.#",
        "#########.",
        ".######...",
        "##########",
    ]),
];

fn cells(picture: &[&str; N]) -> Vec<Vec<bool>> {
    picture.iter().map(|row| row.chars().map(|c| c == '#' || c == '~').collect()).collect()
}

/// Indices d'une ligne : longueurs des suites de cases noires (`[0]` pour une ligne vide).
fn clues(line: impl Iterator<Item = bool>) -> Vec<u32> {
    let mut out = Vec::new();
    let mut run = 0;
    for filled in line {
        if filled {
            run += 1;
        } else if run > 0 {
            out.push(run);
            run = 0;
        }
    }
    if run > 0 {
        out.push(run);
    }
    if out.is_empty() {
        out.push(0);
    }
    out
}

/// Marge blanche de chaque côté du papier, en points.
const MARGIN: i64 = 8;
/// Écart entre les indices des lignes et la grille.
const SEP: i64 = 10;
/// Écart entre deux nombres d'une même ligne d'indices.
const ROW_GAP: i64 = 10;
/// Côté maximal d'une case : au-delà, la grille ne gagne plus en lisibilité.
const MAX_CELL: i64 = 44;

/// Largeur des indices d'une ligne, à l'échelle `scale`.
fn row_width(clue: &[u32], scale: i64) -> i64 {
    clue.iter().map(|&n| draw::number_width(n, scale)).sum::<i64>() + (clue.len() as i64 - 1) * ROW_GAP
}

/// Échelle des chiffres et côté des cases : les plus grands qui tiennent dans la largeur du papier,
/// avec des indices de colonne qui tiennent dans une case.
fn layout(rows: &[Vec<u32>], cols: &[Vec<u32>]) -> (i64, i64) {
    for scale in [3, 2] {
        let clues_width = rows.iter().map(|c| row_width(c, scale)).max().unwrap_or(0);
        let widest = cols.iter().flatten().map(|&n| draw::number_width(n, scale)).max().unwrap_or(0);
        let cell = ((512 - 2 * MARGIN - SEP - clues_width) / N as i64).min(MAX_CELL);
        if cell >= widest + 6 && cell >= draw::DIGIT_H * scale + 6 {
            return (scale, cell);
        }
    }
    (2, 30)
}

fn render(grid: &[Vec<bool>], solution: bool) -> GrayImage {
    let rows: Vec<Vec<u32>> = grid.iter().map(|r| clues(r.iter().copied())).collect();
    let cols: Vec<Vec<u32>> = (0..N).map(|x| clues(grid.iter().map(|r| r[x]))).collect();
    let (scale, cell) = layout(&rows, &cols);
    let clue_h = draw::DIGIT_H * scale + 6;
    let clues_width = rows.iter().map(|c| row_width(c, scale)).max().unwrap_or(0);
    let size = cell * N as i64;
    // L'ensemble (indices des lignes + grille) est centré sur le papier.
    let left = (512 - (clues_width + SEP + size)) / 2;
    let top = cols.iter().map(Vec::len).max().unwrap_or(1) as i64 * clue_h + 4;
    let (ox, oy) = (left + clues_width + SEP, top);
    let mut img = draw::canvas((oy + size + 10) as u32);

    // Indices des colonnes, empilés au-dessus de la grille, alignés en bas.
    for (x, clue) in cols.iter().enumerate() {
        let cx = ox + x as i64 * cell + cell / 2;
        for (k, &n) in clue.iter().rev().enumerate() {
            let y = oy - (k as i64 + 1) * clue_h;
            draw::number(&mut img, n, cx - draw::number_width(n, scale) / 2, y, scale);
        }
    }
    // Indices des lignes, à gauche, alignés à droite contre la grille.
    for (y, clue) in rows.iter().enumerate() {
        let cy = oy + y as i64 * cell + (cell - draw::DIGIT_H * scale) / 2;
        let mut x = ox - SEP;
        for &n in clue.iter().rev() {
            x -= draw::number_width(n, scale);
            draw::number(&mut img, n, x, cy, scale);
            x -= ROW_GAP;
        }
    }
    if solution {
        for (y, row) in grid.iter().enumerate() {
            for (x, &filled) in row.iter().enumerate() {
                if filled {
                    draw::fill_rect(&mut img, ox + x as i64 * cell, oy + y as i64 * cell, cell, cell);
                }
            }
        }
    }
    // Quadrillage : traits épais tous les cinq, cadre compris.
    for k in 0..=N as i64 {
        let thick = if k % 5 == 0 { 5 } else { 2 };
        draw::fill_rect(&mut img, ox + k * cell - thick / 2, oy - thick / 2, thick, size + thick);
        draw::fill_rect(&mut img, ox - thick / 2, oy + k * cell - thick / 2, size + thick, thick);
    }
    img
}

pub fn build(number: Option<usize>, solution: bool) -> anyhow::Result<Doc> {
    let index = match number {
        Some(n) if (1..=PICTURES.len()).contains(&n) => n - 1,
        Some(n) => anyhow::bail!("logimage n° {n} inconnu (1 à {})", PICTURES.len()),
        None => rand::rng().random_range(0..PICTURES.len()),
    };
    let (name, picture) = &PICTURES[index];
    let grid = cells(picture);

    let mut doc = Doc::new();
    doc.header(if solution { "Solution du logimage" } else { "Logimage" });
    if solution {
        doc.text(&format!("C'était… {name} !"), Style::default().small().center());
    } else {
        doc.text("Noircis les cases : les nombres donnent les suites de cases noires, dans l'ordre.", Style::default().small().center());
    }
    doc.feed(1);
    doc.image(render(&grid, solution));
    doc.text(&format!("n° {}", index + 1), Style::default().small().align(Align::Right));
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_picture_fits_the_paper_with_big_cells() {
        for (name, picture) in PICTURES {
            let grid = cells(picture);
            let rows: Vec<Vec<u32>> = grid.iter().map(|r| clues(r.iter().copied())).collect();
            let cols: Vec<Vec<u32>> = (0..N).map(|x| clues(grid.iter().map(|r| r[x]))).collect();
            let (scale, cell) = layout(&rows, &cols);
            let width = rows.iter().map(|c| row_width(c, scale)).max().unwrap() + SEP + cell * N as i64;
            assert!(width <= 512 - 2 * MARGIN, "{name} : {width} points");
            assert!(cell >= 36, "{name} : cases de {cell} points seulement");
        }
    }

    /// Toutes les façons de placer les suites `clue` dans une ligne compatible avec `known`.
    fn placements(clue: &[u32], known: &[Option<bool>]) -> Vec<Vec<bool>> {
        fn go(clue: &[u32], known: &[Option<bool>], start: usize, line: &mut Vec<bool>, out: &mut Vec<Vec<bool>>) {
            let n = known.len();
            let Some((&first, rest)) = clue.split_first() else {
                if (start..n).all(|i| known[i] != Some(true)) {
                    let mut full = line.clone();
                    full.resize(n, false);
                    out.push(full);
                }
                return;
            };
            let len = first as usize;
            let needed: usize = rest.iter().map(|&r| r as usize + 1).sum();
            for pos in start..=n.saturating_sub(len + needed) {
                if (start..pos).any(|i| known[i] == Some(true)) {
                    break;
                }
                if (pos..pos + len).any(|i| known[i] == Some(false)) {
                    continue;
                }
                let end = pos + len;
                if end < n && known[end] == Some(true) {
                    continue;
                }
                let mark = line.len();
                line.resize(pos, false);
                line.extend(std::iter::repeat_n(true, len));
                if end < n {
                    line.push(false);
                }
                go(rest, known, (end + 1).min(n), line, out);
                line.truncate(mark);
            }
        }
        let clue: &[u32] = if clue == [0] { &[] } else { clue };
        let mut out = Vec::new();
        go(clue, known, 0, &mut Vec::new(), &mut out);
        out
    }

    /// Résout par déduction ligne à ligne ; `None` si la grille reste ambiguë.
    fn solve(rows: &[Vec<u32>], cols: &[Vec<u32>]) -> Option<Vec<Vec<bool>>> {
        let mut grid = vec![vec![None; N]; N];
        loop {
            let mut changed = false;
            for vertical in [false, true] {
                for i in 0..N {
                    let line: Vec<Option<bool>> =
                        (0..N).map(|j| if vertical { grid[j][i] } else { grid[i][j] }).collect();
                    let options = placements(if vertical { &cols[i] } else { &rows[i] }, &line);
                    assert!(!options.is_empty(), "grille contradictoire");
                    for j in 0..N {
                        if line[j].is_none() && options.iter().all(|o| o[j] == options[0][j]) {
                            let cell = if vertical { &mut grid[j][i] } else { &mut grid[i][j] };
                            *cell = Some(options[0][j]);
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        grid.into_iter().map(|row| row.into_iter().collect()).collect()
    }

    #[test]
    fn every_picture_has_a_unique_logical_solution() {
        let mut ambiguous = Vec::new();
        for (name, picture) in PICTURES {
            assert!(picture.iter().all(|r| r.chars().count() == N), "{name} : ligne de mauvaise taille");
            let grid = cells(picture);
            let rows: Vec<Vec<u32>> = grid.iter().map(|r| clues(r.iter().copied())).collect();
            let cols: Vec<Vec<u32>> = (0..N).map(|x| clues(grid.iter().map(|r| r[x]))).collect();
            if solve(&rows, &cols).as_ref() != Some(&grid) {
                ambiguous.push(*name);
            }
        }
        assert!(ambiguous.is_empty(), "solution ambiguë : {ambiguous:?}");
    }

    #[test]
    fn clues_count_runs() {
        assert_eq!(clues("##.#..###.".chars().map(|c| c == '#')), vec![2, 1, 3]);
        assert_eq!(clues("..........".chars().map(|c| c == '#')), vec![0]);
    }
}
