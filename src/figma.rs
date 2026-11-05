//! Figma detection (from window titles) and the presence state machine.

use crate::discord::Activity;
use crate::platform::{self, Window};

/// No keyboard/mouse input, or Figma in the background, for this long = idle.
pub const DEFAULT_IDLE_AFTER_SECS: u64 = 5 * 60;
/// Discord rejects "details" longer than 128 characters.
const MAX_DETAILS_LEN: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Desktop,
    Browser,
}

impl Platform {
    fn label(self) -> &'static str {
        match self {
            Platform::Desktop => "Figma Desktop",
            Platform::Browser => "Figma Browser",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FigmaWindow {
    pub focused: bool,
    /// File name, or None when no file is open / the title isn't readable
    pub project: Option<String>,
    pub platform: Platform,
}

// ---------------------------------------------------------------------------
// Window classification
// ---------------------------------------------------------------------------

fn is_desktop(w: &Window) -> bool {
    let owner = w.owner.to_lowercase();
    owner == "figma" || owner == "figma beta"
}

fn is_browser(w: &Window) -> bool {
    const BROWSERS: [&str; 7] = ["chrome", "msedge", "edge", "firefox", "safari", "brave", "opera"];
    const EXACT: [&str; 4] = ["arc", "vivaldi", "zen", "zen browser"];
    let owner = w.owner.to_lowercase();
    BROWSERS.iter().any(|b| owner.contains(b)) || EXACT.contains(&owner.as_str())
}

fn classify(w: &Window, focused: bool) -> Option<FigmaWindow> {
    let platform = if is_desktop(w) {
        Platform::Desktop
    } else if is_browser(w) && strip_figma_suffix(&w.title).is_some() {
        Platform::Browser
    } else {
        return None;
    };
    Some(FigmaWindow { focused, project: project_name(&w.title), platform })
}

/// Find Figma: the focused window first, then any open window, then the app process.
pub fn find(previous_project: Option<&str>) -> Option<FigmaWindow> {
    if let Some(found) = platform::active_window().and_then(|w| classify(&w, true)) {
        return Some(found);
    }

    let open: Vec<FigmaWindow> = platform::open_windows().iter().filter_map(|w| classify(w, false)).collect();
    if !open.is_empty() {
        // Prefer the file we were already showing so the status doesn't flip between windows
        return open
            .iter()
            .find(|f| previous_project.is_some() && f.project.as_deref() == previous_project)
            .or(open.first())
            .cloned();
    }

    // No visible window (e.g. minimized on macOS) but the app is still open
    if platform::figma_app_running() {
        return Some(FigmaWindow {
            focused: false,
            project: previous_project.map(str::to_owned),
            platform: Platform::Desktop,
        });
    }
    None
}

// ---------------------------------------------------------------------------
// Title parsing
// ---------------------------------------------------------------------------

/// Figma titles end in " – Figma" / " - Figma", optionally followed by the browser/profile:
///   "My file – Figma - Google Chrome"
///   "My file – Figma and 2 more pages - Personal - Microsoft Edge"
/// Returns the part before the suffix, or None if the title has no Figma suffix
/// (so pages that merely mention Figma don't match).
fn strip_figma_suffix(title: &str) -> Option<&str> {
    let chars: Vec<(usize, char)> = title.char_indices().collect();
    let is_dash = |c: char| matches!(c, '-' | '–' | '—');

    for i in 0..chars.len() {
        if !word_at(&chars, i, "figma") {
            continue;
        }
        // Backward: whitespace+, dash, whitespace+
        let mut j = i;
        let ws_after_dash = skip_ws_back(&chars, &mut j);
        if ws_after_dash == 0 || j == 0 || !is_dash(chars[j - 1].1) {
            continue;
        }
        j -= 1;
        if skip_ws_back(&chars, &mut j) == 0 {
            continue;
        }

        // Forward: optional " and N more page(s)", then end or " - anything"
        let after = i + "figma".len();
        let rest_ok = |p: usize| {
            let mut k = p;
            let ws = skip_ws(&chars, &mut k);
            k == chars.len() || (ws > 0 && is_dash(chars[k].1) && { k += 1; skip_ws(&chars, &mut k) > 0 })
        };
        let matched = more_pages(&chars, after).is_some_and(|p| rest_ok(p)) || rest_ok(after);
        if matched {
            return Some(title[..chars[j].0].trim_end());
        }
    }
    None
}

fn word_at(chars: &[(usize, char)], i: usize, word: &str) -> bool {
    let mut k = i;
    for w in word.chars() {
        match chars.get(k) {
            Some(&(_, c)) if c.to_ascii_lowercase() == w => k += 1,
            _ => return false,
        }
    }
    true
}

fn skip_ws(chars: &[(usize, char)], k: &mut usize) -> usize {
    let start = *k;
    while *k < chars.len() && chars[*k].1.is_whitespace() {
        *k += 1;
    }
    *k - start
}

fn skip_ws_back(chars: &[(usize, char)], j: &mut usize) -> usize {
    let start = *j;
    while *j > 0 && chars[*j - 1].1.is_whitespace() {
        *j -= 1;
    }
    start - *j
}

/// Matches `\s+and\s+\d+\s+more\s+pages?` at `p`, returning the index after it.
fn more_pages(chars: &[(usize, char)], p: usize) -> Option<usize> {
    let mut k = p;
    if skip_ws(chars, &mut k) == 0 || !word_at(chars, k, "and") {
        return None;
    }
    k += 3;
    if skip_ws(chars, &mut k) == 0 {
        return None;
    }
    let digits = k;
    while k < chars.len() && chars[k].1.is_ascii_digit() {
        k += 1;
    }
    if k == digits || skip_ws(chars, &mut k) == 0 || !word_at(chars, k, "more") {
        return None;
    }
    k += 4;
    if skip_ws(chars, &mut k) == 0 || !word_at(chars, k, "page") {
        return None;
    }
    k += 4;
    if chars.get(k).is_some_and(|&(_, c)| c == 's') {
        k += 1;
    }
    Some(k)
}

/// File name from a window title, or None when there isn't one (home screen, empty title).
fn project_name(title: &str) -> Option<String> {
    let name = strip_figma_suffix(title).unwrap_or(title).trim();
    if name.is_empty() || name.eq_ignore_ascii_case("figma") {
        return None;
    }
    let max = MAX_DETAILS_LEN - "Working on ".len();
    Some(if name.chars().count() > max {
        name.chars().take(max - 1).chain(['…']).collect()
    } else {
        name.to_owned()
    })
}

// ---------------------------------------------------------------------------
// Presence state machine
// ---------------------------------------------------------------------------

struct Session {
    start_ms: u64,
    last_focused_ms: u64,
}

pub struct Tracker {
    idle_after_ms: u64,
    session: Option<Session>,
    pub project: Option<String>,
}

pub enum Update {
    /// Figma is open: show this activity
    Show { activity: Activity, key: String },
    /// Figma closed (or never opened): clear the status
    Clear,
}

impl Tracker {
    pub fn new(idle_after_secs: u64) -> Self {
        Self { idle_after_ms: idle_after_secs * 1000, session: None, project: None }
    }

    /// `input_idle_secs`: seconds since the last keyboard/mouse input anywhere.
    pub fn update(&mut self, figma: Option<FigmaWindow>, input_idle_secs: u64, now_ms: u64) -> Update {
        let Some(figma) = figma else {
            self.session = None;
            self.project = None;
            return Update::Clear;
        };

        // Timer starts when Figma is first seen and only restarts after Figma is closed
        let session = self.session.get_or_insert(Session { start_ms: now_ms, last_focused_ms: now_ms });
        if figma.focused {
            session.last_focused_ms = now_ms;
        }
        let away = now_ms.saturating_sub(session.last_focused_ms) >= self.idle_after_ms;
        let idle = away || input_idle_secs * 1000 >= self.idle_after_ms;

        let name = figma.project.as_deref().unwrap_or("Figma");
        let details = format!("{} {}", if idle { "Idle in" } else { "Working on" }, name);
        let state = if idle {
            format!("{} · Idle", figma.platform.label())
        } else {
            figma.platform.label().to_owned()
        };

        self.project = figma.project.clone();
        Update::Show {
            key: format!("{details}|{state}"),
            activity: Activity { details, state, start_ms: session.start_ms, large_text: figma.platform.label().into() },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_titles() {
        let cases = [
            ("InfinityInvoice – hero art - Figma", Some("InfinityInvoice – hero art")),
            ("Untitled – Figma - Google Chrome", Some("Untitled")),
            ("Landing page – Figma and 2 more pages - Personal - Microsoft\u{200b} Edge", Some("Landing page")),
            ("Mobile app – Figma — Mozilla Firefox", Some("Mobile app")),
            ("Drafts – Figma - Google Chrome - Krutik", Some("Drafts")),
            ("Single – Figma and 1 more page - Microsoft Edge", Some("Single")),
            ("krutik/FigTime: displays your Figma activity status on Discord. - Google Chrome", None),
            ("Figma vs Sketch - Google Search - Google Chrome", None),
            ("How to use Figma - YouTube - Google Chrome", None),
            ("Figma", None),
            ("Figmas - Google Chrome", None),
        ];
        for (title, expected) in cases {
            assert_eq!(strip_figma_suffix(title), expected, "{title}");
        }
    }

    #[test]
    fn project_names() {
        assert_eq!(project_name("Hero – Figma"), Some("Hero".into()));
        assert_eq!(project_name("Figma"), None);
        assert_eq!(project_name(""), None);
        let long = format!("{} - Figma", "x".repeat(200));
        let name = project_name(&long).unwrap();
        assert_eq!(name.chars().count(), MAX_DETAILS_LEN - "Working on ".len());
        assert!(name.ends_with('…'));
    }

    fn win(focused: bool) -> Option<FigmaWindow> {
        Some(FigmaWindow { focused, project: Some("Hero".into()), platform: Platform::Desktop })
    }

    fn show(u: Update) -> Activity {
        match u {
            Update::Show { activity, .. } => activity,
            Update::Clear => panic!("expected Show"),
        }
    }

    #[test]
    fn idle_and_timer() {
        let mut t = Tracker::new(300);
        let a = show(t.update(win(true), 0, 1_000));
        assert_eq!(a.details, "Working on Hero");
        assert_eq!(a.start_ms, 1_000);

        // No input for 5 min -> idle, same timer
        let a = show(t.update(win(true), 300, 2_000));
        assert_eq!(a.details, "Idle in Hero");
        assert_eq!(a.state, "Figma Desktop · Idle");
        assert_eq!(a.start_ms, 1_000);

        // Back to work
        assert_eq!(show(t.update(win(true), 0, 3_000)).details, "Working on Hero");

        // Figma in background: not idle yet, then idle after 5 min
        assert_eq!(show(t.update(win(false), 0, 3_000 + 299_000)).details, "Working on Hero");
        assert_eq!(show(t.update(win(false), 0, 3_000 + 300_000)).details, "Idle in Hero");

        // Closed -> clear; reopened -> new timer
        assert!(matches!(t.update(None, 0, 400_000), Update::Clear));
        assert_eq!(show(t.update(win(true), 0, 500_000)).start_ms, 500_000);
    }
}
