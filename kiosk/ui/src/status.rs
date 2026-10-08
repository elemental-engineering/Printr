//! État de l'appareil, fourni par la carte : horloge (RTC) et Wi-Fi.

/// Date et heure de l'horloge de la carte, telles quelles (pour en vérifier la dérive).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

const DAYS: [&str; 7] = ["Dimanche", "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi"];
const MONTHS: [&str; 12] = [
    "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre",
    "décembre",
];

impl Clock {
    /// Jour de la semaine, 0 pour dimanche (méthode de Sakamoto).
    fn weekday(&self) -> usize {
        const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let m = usize::from(self.month.clamp(1, 12)) - 1;
        let y = if m < 2 { self.year - 1 } else { self.year };
        (y + y / 4 - y / 100 + y / 400 + T[m] + i32::from(self.day)).rem_euclid(7) as usize
    }

    /// « Jeudi 8 octobre 2026 ».
    pub fn date(&self) -> String {
        let month = MONTHS[usize::from(self.month.clamp(1, 12)) - 1];
        format!("{} {} {month} {}", DAYS[self.weekday()], self.day, self.year)
    }

    /// « 22:31:05 ».
    pub fn time(&self) -> String {
        format!("{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
    }
}

/// Wi-Fi de la carte.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum WifiStatus {
    /// Éteint, ou pas de réseau : icône barrée.
    #[default]
    Off,
    /// Recherche du réseau ou connexion en cours.
    Searching,
    /// Connecté, avec la force du signal en dBm et l'adresse obtenue.
    Connected { rssi: i8, ip: Option<String> },
}

impl WifiStatus {
    /// Nombre d'arcs allumés de l'icône, de 0 (le point seul) à 3.
    pub fn bars(&self) -> u8 {
        match self {
            WifiStatus::Connected { rssi, .. } => match rssi {
                -55.. => 3,
                -67..=-56 => 2,
                -78..=-68 => 1,
                _ => 0,
            },
            _ => 0,
        }
    }

    /// Qualité du signal, en mots.
    pub fn quality(&self) -> &'static str {
        match self.bars() {
            3 => "excellent",
            2 => "bon",
            1 => "moyen",
            _ => "faible",
        }
    }
}

/// Synchronisation de l'horloge par NTP, faite par la carte une fois le Wi-Fi connecté.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NtpStatus {
    /// Pas encore tentée.
    #[default]
    Idle,
    Syncing,
    /// Horloge mise à l'heure ; `offset_ms` est l'écart actuel horloge − NTP, mis à jour en
    /// continu par la carte pour suivre la dérive depuis la synchronisation.
    Synced { offset_ms: i64 },
    Failed,
}

/// Écart signé, lisible : « +12 ms », « -1,234 s », « +2 min 05 s ».
pub fn format_offset(ms: i64) -> String {
    // Tiret ASCII : le signe moins typographique n'existe pas dans les polices de l'écran.
    let sign = if ms < 0 { "-" } else { "+" };
    let abs = ms.unsigned_abs();
    match abs {
        0..1_000 => format!("{sign}{abs} ms"),
        1_000..60_000 => format!("{sign}{},{:03} s", abs / 1000, abs % 1000),
        _ => format!("{sign}{} min {:02} s", abs / 60_000, abs / 1000 % 60),
    }
}

/// Icône Wi-Fi, 15 × 12 : `0` le point, `1` à `3` les arcs, du plus petit au plus grand.
pub const WIFI_ICON: [&str; 12] = [
    "....3333333....",
    "..33.......33..",
    ".3...........3.",
    "3...2222222...3",
    "...2.......2...",
    "..2.........2..",
    ".....11111.....",
    "....1.....1....",
    "...............",
    "......000......",
    "......000......",
    "...............",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn french_date_and_time() {
        let clock = Clock { year: 2026, month: 10, day: 8, hour: 22, minute: 3, second: 5 };
        assert_eq!(clock.date(), "Jeudi 8 octobre 2026");
        assert_eq!(clock.time(), "22:03:05");
        let new_year = Clock { year: 2027, month: 1, day: 1, hour: 0, minute: 0, second: 0 };
        assert_eq!(new_year.date(), "Vendredi 1 janvier 2027");
    }

    #[test]
    fn offsets() {
        assert_eq!(format_offset(12), "+12 ms");
        assert_eq!(format_offset(-1234), "-1,234 s");
        assert_eq!(format_offset(125_400), "+2 min 05 s");
        assert_eq!(format_offset(0), "+0 ms");
    }

    #[test]
    fn signal_bars() {
        let at = |rssi| WifiStatus::Connected { rssi, ip: None }.bars();
        assert_eq!([at(-40), at(-60), at(-70), at(-85)], [3, 2, 1, 0]);
        assert_eq!(WifiStatus::Searching.bars(), 0);
    }

    #[test]
    fn wifi_icon_rows_are_even() {
        assert!(WIFI_ICON.iter().all(|row| row.len() == 15));
    }
}
