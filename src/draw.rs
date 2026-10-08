//! Primitives de dessin sur image noir et blanc (sudoku, labyrinthe, mots mêlés, voie ferrée…).

use image::{GrayImage, Luma};

use crate::raster::PRINT_WIDTH;

pub const BLACK: Luma<u8> = Luma([0]);
pub const WHITE: Luma<u8> = Luma([255]);

/// Canevas blanc pleine largeur.
pub fn canvas(height: u32) -> GrayImage {
    GrayImage::from_pixel(PRINT_WIDTH, height, Luma([255]))
}

/// Rectangle plein, rogné aux bords de l'image.
pub fn fill_rect(img: &mut GrayImage, x: i64, y: i64, w: i64, h: i64) {
    fill_rect_with(img, x, y, w, h, BLACK);
}

/// Rectangle plein de la couleur `ink`, rogné aux bords de l'image.
pub fn fill_rect_with(img: &mut GrayImage, x: i64, y: i64, w: i64, h: i64, ink: Luma<u8>) {
    let (iw, ih) = (img.width() as i64, img.height() as i64);
    for py in y.max(0)..(y + h).min(ih) {
        for px in x.max(0)..(x + w).min(iw) {
            img.put_pixel(px as u32, py as u32, ink);
        }
    }
}

/// Chiffres 1 à 9 en 5×7.
const DIGITS: [[&str; 7]; 9] = [
    ["..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###."],
    [".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####"],
    ["#####", "...#.", "..#..", "...#.", "....#", "#...#", ".###."],
    ["...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#."],
    ["#####", "#....", "####.", "....#", "....#", "#...#", ".###."],
    ["..##.", ".#...", "#....", "####.", "#...#", "#...#", ".###."],
    ["#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#..."],
    [".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###."],
    [".###.", "#...#", "#...#", ".####", "....#", "...#.", ".##.."],
];

/// Le zéro, à part : les grilles de sudoku n'en ont pas besoin.
const ZERO: [&str; 7] = [".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###."];

pub const DIGIT_W: i64 = 5;
pub const DIGIT_H: i64 = 7;

/// Dessine le chiffre `d` (1..=9) à l'échelle `scale`, coin haut-gauche en (x, y).
pub fn digit(img: &mut GrayImage, d: u8, x: i64, y: i64, scale: i64) {
    let glyph = &DIGITS[(d - 1) as usize];
    for (row, line) in glyph.iter().enumerate() {
        for (col, c) in line.bytes().enumerate() {
            if c == b'#' {
                fill_rect(img, x + col as i64 * scale, y + row as i64 * scale, scale, scale);
            }
        }
    }
}

/// Dessine un nombre entier (chiffres de 5×7 séparés d'une colonne), coin haut-gauche en (x, y).
pub fn number(img: &mut GrayImage, n: u32, x: i64, y: i64, scale: i64) {
    for (i, d) in n.to_string().bytes().enumerate() {
        let x = x + i as i64 * (DIGIT_W + 1) * scale;
        let d = d - b'0';
        if d == 0 {
            glyph(img, &ZERO, x, y, scale, BLACK);
        } else {
            digit(img, d, x, y, scale);
        }
    }
}

/// Largeur, en points, d'un nombre dessiné par `number`.
pub fn number_width(n: u32, scale: i64) -> i64 {
    let len = n.to_string().len() as i64;
    (len * (DIGIT_W + 1) - 1) * scale
}

fn glyph(img: &mut GrayImage, rows: &[&str; 7], x: i64, y: i64, scale: i64, ink: Luma<u8>) {
    for (row, line) in rows.iter().enumerate() {
        for (col, b) in line.bytes().enumerate() {
            if b == b'#' {
                fill_rect_with(img, x + col as i64 * scale, y + row as i64 * scale, scale, scale, ink);
            }
        }
    }
}

/// Lettres A à Z en 5×7, même dessin que les chiffres.
const LETTERS: [[&str; 7]; 26] = [
    [".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
    ["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."],
    [".###.", "#...#", "#....", "#....", "#....", "#...#", ".###."],
    ["####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####."],
    ["#####", "#....", "#....", "####.", "#....", "#....", "#####"],
    ["#####", "#....", "#....", "####.", "#....", "#....", "#...."],
    [".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".####"],
    ["#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
    [".###.", "..#..", "..#..", "..#..", "..#..", "..#..", ".###."],
    ["..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##.."],
    ["#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#"],
    ["#....", "#....", "#....", "#....", "#....", "#....", "#####"],
    ["#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#"],
    ["#...#", "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#"],
    [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
    ["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."],
    [".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#"],
    ["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"],
    [".####", "#....", "#....", ".###.", "....#", "....#", "####."],
    ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."],
    ["#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
    ["#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#.."],
    ["#...#", "#...#", "#...#", "#.#.#", "#.#.#", "#.#.#", ".#.#."],
    ["#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#"],
    ["#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#.."],
    ["#####", "....#", "...#.", "..#..", ".#...", "#....", "#####"],
];

/// Dessine la lettre majuscule `c` (A..=Z) à l'échelle `scale`, dans la couleur `ink`.
pub fn letter(img: &mut GrayImage, c: u8, x: i64, y: i64, scale: i64, ink: Luma<u8>) {
    let glyph = &LETTERS[(c - b'A') as usize];
    for (row, line) in glyph.iter().enumerate() {
        for (col, b) in line.bytes().enumerate() {
            if b == b'#' {
                fill_rect_with(img, x + col as i64 * scale, y + row as i64 * scale, scale, scale, ink);
            }
        }
    }
}
