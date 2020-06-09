const Emptys: &[&str] = &[
    "\u{0009}", "\u{0085}", "\u{00A0}", "\u{1680}", "\u{2000}", "\u{2001}", "\u{2002}", "\u{2003}",
    "\u{2004}", "\u{2005}", "\u{2006}", "\u{2007}", "\u{2008}", "\u{2009}", "\u{200A}", "\u{202F}",
    "\u{205F}", "\u{3000}", "\u{3164}", "\u{180E}", "\u{200B}", "\u{200C}", "\u{200D}", "\u{2060}",
    "\u{FEFF}",
];

pub fn trim_empty(s: &str) -> String {
    let mut t = s.to_string();
    for empty in Emptys {
        if t.contains(empty) {
            println!("Hits empty: {}", empty);
            t = t.replace(empty, "");
        }
    }
    t
}
