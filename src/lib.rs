use crossterm::cursor::{self, MoveTo, MoveToColumn, Show};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, read};
use crossterm::terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode};
use crossterm::{QueueableCommand, terminal};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use item::Item;
use list::List;
use pastel_colours::{
    BLUE_FG, DARK_BLUE_BG, DARK_GREY_BG, DARK_GREY_FG, GREEN_FG, RESET_BG, RESET_FG,
};
use std::io::{Stdout, Write, stdout};

pub mod item;
mod list;

/// The error type for fuzzy_finder. Everything the terminal does
/// surfaces as I/O errors, but having our own type means we can
/// extend it without breaking callers.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("terminal error: {0}")]
    Terminal(#[from] std::io::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Puts the terminal into raw mode, and guarantees it's restored when
/// dropped — including on error paths and panics.
struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

pub struct FuzzyFinder<T>
where
    T: Clone,
{
    search_term: String,
    all_items: Vec<Item<T>>,
    matches: Vec<Item<T>>,
    /// The 0-based terminal row where the list starts rendering.
    start_row: u16,
    stdout: Stdout,
    first: bool,
    list: List<T>,
}

impl<T> FuzzyFinder<T>
where
    T: Clone,
{
    fn new(items: Vec<Item<T>>, lines_to_show: u16) -> Result<Self> {
        let stdout = stdout();

        // We need to know where to start rendering from. If the list would
        // overflow the bottom of the terminal then the initial padding
        // (see `render_space`) will scroll everything up, so account for that.
        let cursor_row = match cursor::position() {
            Ok((_, row)) => row,
            Err(err) => {
                log::error!("Cannot get cursor position, assuming top of screen: {err}");
                0
            }
        };
        let (_, terminal_height) = terminal::size()?;
        let overflow = (cursor_row + 1 + lines_to_show).saturating_sub(terminal_height);
        let start_row = cursor_row.saturating_sub(overflow);

        Ok(FuzzyFinder {
            search_term: String::new(),
            all_items: items,
            matches: vec![],
            start_row,
            stdout,
            first: true,
            list: List::new(lines_to_show as usize),
        })
    }

    fn up(&mut self) -> Result<()> {
        self.list.up(&self.matches);
        self.update_matches();
        self.render()
    }

    fn down(&mut self) -> Result<()> {
        self.list.down();
        self.update_matches();
        self.render()
    }

    fn append(&mut self, c: char) -> Result<()> {
        self.list.reset_selection();
        self.search_term.push(c);
        self.update_matches();
        self.render()
    }

    fn backspace(&mut self) -> Result<()> {
        self.list.reset_selection();
        // NB: pop() removes the last char, not the last byte, so this is
        // safe for multi-byte characters.
        self.search_term.pop();
        self.update_matches();
        self.render()
    }

    /// Reserves the vertical space we render into. If the cursor is near the
    /// bottom of the terminal this scrolls everything up to make room.
    fn render_space(&mut self) -> Result<()> {
        if self.first {
            let padding = "\r\n".repeat(self.list.lines_to_show);
            write!(self.stdout, "{padding}")?;
            self.first = false;
        }
        Ok(())
    }

    fn render_items(&mut self) -> Result<()> {
        self.stdout.queue(MoveTo(0, self.start_row))?;
        for (index, item) in self.list.items.iter().enumerate() {
            self.stdout
                .queue(Clear(ClearType::CurrentLine))?
                .queue(MoveToColumn(0))?;
            if item.is_blank {
                write!(self.stdout, "\r\n")?;
            } else {
                // Do some string manipulation to colourise the indexed parts
                let fuzzy_indices = item.score.as_ref().map_or(&[] as &[usize], |s| &s.1);
                let coloured_line =
                    get_coloured_line(fuzzy_indices, &item.name, index == self.list.selected_index);
                write!(self.stdout, "{coloured_line}\r\n")?;
            }
        }
        Ok(())
    }

    fn render_prompt(&mut self) -> Result<()> {
        let prompt_row = self.start_row + self.list.lines_to_show as u16;
        self.stdout
            .queue(MoveTo(0, prompt_row))?
            .queue(Clear(ClearType::CurrentLine))?
            .queue(Show)?;
        write!(self.stdout, "{BLUE_FG}${RESET_FG} {}", self.search_term)?;
        self.stdout.flush()?;
        Ok(())
    }

    /// Gets items that match our current criteria, sorted by score.
    fn update_matches(&mut self) {
        self.matches = rank_items(&mut self.all_items, &self.search_term);
        log::info!(
            "There are a total of {} item(s) and {} match(es)",
            self.all_items.len(),
            self.matches.len()
        );
        self.list.update(&self.matches);
    }

    /// Renders the current result set.
    fn render(&mut self) -> Result<()> {
        self.render_space()?;
        self.render_items()?;
        self.render_prompt()?;
        Ok(())
    }

    /// Clears everything we've rendered and puts the cursor back at the
    /// start of the rendered area, so the caller's output follows on neatly.
    fn cleanup(&mut self) -> Result<()> {
        let end_row = self.start_row + self.list.lines_to_show as u16;
        for row in self.start_row..=end_row {
            self.stdout
                .queue(MoveTo(0, row))?
                .queue(Clear(ClearType::CurrentLine))?;
        }
        self.stdout.queue(MoveTo(0, self.start_row))?.queue(Show)?;
        self.stdout.flush()?;
        Ok(())
    }

    /// The main entry point for the fuzzy finder. Renders the finder,
    /// blocks until the user selects an item or quits (Esc, ctrl-c or
    /// ctrl-d), and returns the selection.
    pub fn find(items: Vec<Item<T>>, lines_to_show: u16) -> Result<Option<T>> {
        let _raw_mode = RawModeGuard::new()?;
        let mut state = FuzzyFinder::new(items, lines_to_show)?;

        state.update_matches();
        state.render()?;

        loop {
            let Event::Key(key) = read()? else {
                continue;
            };
            // Windows sends both press and release events; only act on press.
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Esc => break,
                KeyCode::Char('c') | KeyCode::Char('d')
                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    break;
                }
                KeyCode::Enter => {
                    // If there are no matches the selection is a blank,
                    // and we return None — same as quitting.
                    let selected = state
                        .list
                        .get_selected()
                        .and_then(|selected| selected.item.clone());
                    state.cleanup()?;
                    return Ok(selected);
                }
                KeyCode::Up => state.up()?,
                KeyCode::Down => state.down()?,
                KeyCode::Backspace => state.backspace()?,
                KeyCode::Char(c) => state.append(c)?,
                _ => {}
            }
        }

        state.cleanup()?;
        Ok(None)
    }
}

/// Scores every item against the search term, and returns the matching
/// items sorted by score, best match first.
fn rank_items<T: Clone>(all_items: &mut [Item<T>], search_term: &str) -> Vec<Item<T>> {
    let matcher = SkimMatcherV2::default();
    for item in all_items.iter_mut() {
        item.score = matcher.fuzzy_indices(&item.name, search_term);
    }
    let mut matches: Vec<Item<T>> = all_items
        .iter()
        .filter(|item| item.score.is_some())
        .cloned()
        .collect();

    // We want these in the order of their fuzzy matched score, i.e. closest matches
    matches.sort_by(|a, b| b.score.cmp(&a.score));
    matches
}

/// Highlights the line. Will highlight matching search items, and also indicate
/// if it's a selected item.
fn get_coloured_line(fuzzy_indices: &[usize], text: &str, is_selected: bool) -> String {
    // Do some string manipulation to colourise the indexed parts
    let mut coloured_line = String::new();
    let mut start = 0;

    let text_vec = text.chars().collect::<Vec<_>>();
    for i in fuzzy_indices {
        let part = &text_vec[start..*i].iter().collect::<String>();
        let matching_char = &text_vec[*i..*i + 1].iter().collect::<String>();
        if is_selected {
            coloured_line = format!(
                "{coloured_line}{DARK_GREY_BG}{part}{RESET_BG}{DARK_BLUE_BG}{matching_char}{RESET_BG}"
            );
        } else {
            coloured_line = format!("{coloured_line}{part}{DARK_BLUE_BG}{matching_char}{RESET_BG}");
        }
        start = i + 1;
    }
    let remaining_chars = &text_vec[start..text_vec.len()].iter().collect::<String>();
    if is_selected {
        let prompt: String = format!("{DARK_GREY_BG}{GREEN_FG}>{RESET_FG}{RESET_BG}");
        let spacer: String = format!("{DARK_GREY_FG}  {RESET_FG}");
        let remaining: String = format!("{DARK_GREY_BG}{remaining_chars}{RESET_BG}");
        coloured_line = format!("{prompt}{spacer}{coloured_line}{remaining}");
    } else {
        coloured_line = format!("{DARK_GREY_BG} {RESET_BG}  {coloured_line}{remaining_chars}");
    }
    coloured_line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(names: &[&str]) -> Vec<Item<&'static str>> {
        names
            .iter()
            .map(|name| Item::new((*name).to_string(), "payload"))
            .collect()
    }

    fn names<T: Clone>(matches: &[Item<T>]) -> Vec<String> {
        matches.iter().map(|item| item.name.clone()).collect()
    }

    #[test]
    fn rank_items_empty_search_matches_everything() {
        let mut all = items(&["alpha", "beta", "gamma"]);
        let matches = rank_items(&mut all, "");
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn rank_items_filters_non_matches() {
        let mut all = items(&["alpha", "beta", "gamma"]);
        let matches = rank_items(&mut all, "alp");
        assert_eq!(names(&matches), vec!["alpha"]);
    }

    #[test]
    fn rank_items_puts_best_match_first() {
        let mut all = items(&["some/deep/path/thing", "thing"]);
        let matches = rank_items(&mut all, "thing");
        // The exact match should out-score the scattered one.
        assert_eq!(names(&matches)[0], "thing");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn rank_items_records_match_indices() {
        let mut all = items(&["abc"]);
        let matches = rank_items(&mut all, "ac");
        let (_, indices) = matches[0].score.as_ref().expect("expected a score");
        assert_eq!(indices, &vec![0, 2]);
    }

    #[test]
    fn rank_items_is_case_insensitive_by_default() {
        let mut all = items(&["Alpha"]);
        let matches = rank_items(&mut all, "alpha");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn coloured_line_contains_all_text() {
        let line = get_coloured_line(&[0, 2], "abc", false);
        // The characters all survive the colourisation, in order.
        let stripped: String = line
            .chars()
            .filter(|c| "abc".contains(*c) && *c != ' ')
            .collect();
        assert_eq!(stripped, "abc");
    }

    #[test]
    fn coloured_line_marks_selection() {
        let unselected = get_coloured_line(&[], "abc", false);
        let selected = get_coloured_line(&[], "abc", true);
        assert!(!unselected.contains('>'));
        assert!(selected.contains('>'));
    }

    #[test]
    fn coloured_line_handles_multibyte_characters() {
        // 'é' is two bytes; the indices are char indices, not byte indices.
        let line = get_coloured_line(&[1], "éa", false);
        assert!(line.contains('é'));
        assert!(line.contains('a'));
    }

    #[test]
    fn coloured_line_handles_match_on_last_char() {
        let line = get_coloured_line(&[2], "abc", false);
        assert!(line.contains('c'));
    }
}
