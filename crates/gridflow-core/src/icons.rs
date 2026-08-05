//! Named icon glyphs for `icon=NAME`. Names resolve to unicode/emoji glyphs
//! that render as ordinary text everywhere (canvas, SVG, PNG) — no image
//! assets, no separate raster path. `icon="⚙"` (any literal string) bypasses
//! the table for glyphs not listed here.

/// Sorted by name for the docs table; lookup is linear (the table is tiny).
pub const ICONS: &[(&str, &str)] = &[
    ("alert", "⚠"),
    ("api", "⇄"),
    ("bolt", "⚡"),
    ("book", "📖"),
    ("box", "📦"),
    ("bug", "🐛"),
    ("build", "🔨"),
    ("cache", "🗃"),
    ("calendar", "📅"),
    ("camera", "📷"),
    ("chart", "📊"),
    ("chat", "💬"),
    ("check", "✔"),
    ("clock", "⏱"),
    ("cloud", "☁"),
    ("code", "⌨"),
    ("config", "🔧"),
    ("cpu", "🖥"),
    ("cross", "✖"),
    ("db", "🗄"),
    ("doc", "📄"),
    ("download", "⬇"),
    ("email", "✉"),
    ("event", "📣"),
    ("file", "📄"),
    ("fire", "🔥"),
    ("flag", "⚑"),
    ("folder", "📁"),
    ("gear", "⚙"),
    ("globe", "🌐"),
    ("heart", "♥"),
    ("home", "⌂"),
    ("idea", "💡"),
    ("key", "🔑"),
    ("link", "🔗"),
    ("lock", "🔒"),
    ("mobile", "📱"),
    ("money", "💰"),
    ("music", "♪"),
    ("pin", "📌"),
    ("queue", "☰"),
    ("robot", "🤖"),
    ("rocket", "🚀"),
    ("search", "🔍"),
    ("server", "🖳"),
    ("shield", "🛡"),
    ("star", "★"),
    ("stop", "🛑"),
    ("sync", "⟳"),
    ("terminal", "⌨"),
    ("timer", "⏳"),
    ("trash", "🗑"),
    ("unlock", "🔓"),
    ("upload", "⬆"),
    ("user", "👤"),
    ("users", "👥"),
    ("warn", "⚠"),
    ("web", "🌐"),
];

/// Resolve an icon reference: a known name maps through the table; anything
/// else is treated as a literal glyph (so `icon="⚙"` and future emoji work).
pub fn glyph(name: &str) -> &str {
    ICONS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, g)| *g)
        .unwrap_or(name)
}

/// Is this a name the table knows? (Used by the resolver to warn on likely
/// typos: multi-char ASCII strings that match nothing are probably mistakes.)
pub fn is_known(name: &str) -> bool {
    ICONS.iter().any(|(n, _)| *n == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_names_resolve() {
        assert_eq!(glyph("gear"), "⚙");
        assert_eq!(glyph("db"), "🗄");
    }

    #[test]
    fn literal_glyphs_pass_through() {
        assert_eq!(glyph("⚙"), "⚙");
        assert_eq!(glyph("λ"), "λ");
    }

    #[test]
    fn table_is_sorted_and_unique() {
        for pair in ICONS.windows(2) {
            assert!(pair[0].0 < pair[1].0, "{} >= {}", pair[0].0, pair[1].0);
        }
    }
}
