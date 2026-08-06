# Change Log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and this project adheres to [Semantic Versioning](https://semver.org/).

## [2026-08-06] - 0.5.0

### Changed

- Ported the terminal handling from `termion` to `crossterm`. Arrow keys,
  Esc, Enter etc. are now proper key events rather than hand-parsed escape
  sequences, so Esc quits instantly and reliably
- Raw mode is now restored via an RAII guard, so the terminal is put back
  even if the caller panics
- `FuzzyFinder::find` takes `lines_to_show: u16` instead of `i8` (breaking)
- Errors are now a `fuzzy_finder::Error` (via `thiserror`) instead of `anyhow`
  (breaking)
- Rust edition 2024, all dependencies updated
- `csv` and `serde` are now dev-dependencies — they were only used by the
  example, and consumers no longer pull them in
- Selecting an item or quitting now clears the rendered lines properly

### Fixes

- Backspace no longer panics on multi-byte (e.g. accented) characters
- Pressing up with no matches no longer scrolls the view into nothing
  (u8 index overflow)

## [2023-01-12] - 0.3.2
### Fixes
  - #1 - Fix issue where the selection/view wasn't reset when match numbers dropped

## [2022-09-16] - 0.3.0

### Fixes
 - Search fix from @BeLeap

