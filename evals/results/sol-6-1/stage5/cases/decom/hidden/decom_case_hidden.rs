//! Assessor-only behavioral checks for the frozen DECOM change task.
//! All inputs use the public escape-sequence processor. Coordinates asserted below are zero-based.

use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, TermMode};
use alacritty_terminal::vte::ansi::{self, Color, NamedColor};
use alacritty_terminal::Term;

struct Size;

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        8
    }

    fn screen_lines(&self) -> usize {
        8
    }

    fn columns(&self) -> usize {
        12
    }
}

struct Terminal {
    term: Term<VoidListener>,
    parser: ansi::Processor,
}

impl Terminal {
    fn new() -> Self {
        Self {
            term: Term::new(Config::default(), &Size, VoidListener),
            parser: ansi::Processor::new(),
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.parser.advance(&mut self.term, byte);
        }
    }

    fn point(&self) -> Point {
        self.term.grid().cursor.point
    }

    fn at(&self, row: i32, column: usize) {
        assert_eq!(self.point(), Point::new(Line(row), Column(column)));
    }

    fn origin(&self) -> bool {
        self.term.mode().contains(TermMode::ORIGIN)
    }

    fn cell(&self, row: i32, column: usize) -> char {
        self.term.grid()[Line(row)][Column(column)].c
    }
}

fn saved_on() -> Terminal {
    let mut terminal = Terminal::new();
    // Save row 4, column 5 of the screen under origin-relative addressing.
    terminal.feed(b"\x1b[3;6r\x1b[?6h\x1b[2;5H\x1b7");
    terminal.at(3, 4);
    terminal.feed(b"\x1b[?6l\x1b[8;10H\x1b8");
    terminal
}

fn saved_off() -> Terminal {
    let mut terminal = Terminal::new();
    // Save a screen-relative row above the scrolling region.
    terminal.feed(b"\x1b[3;6r\x1b[2;5H\x1b7");
    terminal.at(1, 4);
    terminal.feed(b"\x1b[?6h\x1b[3;9H\x1b8");
    terminal
}

// F1: enabling DECOM homes under the newly enabled origin.
#[test]
fn set_homes_nondefault_margins() {
    let mut terminal = Terminal::new();
    terminal.feed(b"\x1b[3;6r\x1b[1;8H\x1b[?6h");
    assert!(terminal.origin());
    terminal.at(2, 0);
}

#[test]
fn set_writes_at_region_home() {
    let mut terminal = Terminal::new();
    terminal.feed(b"\x1b[1;1HA\x1b[8;1HZ\x1b[3;6r\x1b[1;8H\x1b[?6hX");
    assert_eq!(terminal.cell(2, 0), 'X');
    assert_eq!(terminal.cell(0, 7), ' ');
    assert_eq!(terminal.cell(0, 0), 'A');
    assert_eq!(terminal.cell(7, 0), 'Z');
    terminal.at(2, 1);
}

#[test]
fn set_home_clears_pending_wrap() {
    let mut terminal = Terminal::new();
    terminal.feed(b"\x1b[3;6r\x1b[1;12HA");
    assert!(terminal.term.grid().cursor.input_needs_wrap);
    terminal.feed(b"\x1b[?6h");
    assert!(!terminal.term.grid().cursor.input_needs_wrap);
    terminal.feed(b"B");
    assert_eq!(terminal.cell(2, 0), 'B');
    terminal.at(2, 1);
}

// F2: disabling DECOM homes under the newly disabled origin.
#[test]
fn reset_homes_nondefault_margins() {
    let mut terminal = Terminal::new();
    terminal.feed(b"\x1b[3;6r\x1b[?6h\x1b[2;5H\x1b[?6l");
    assert!(!terminal.origin());
    terminal.at(0, 0);
}

#[test]
fn reset_writes_at_screen_home() {
    let mut terminal = Terminal::new();
    terminal.feed(b"\x1b[8;1HZ\x1b[3;6r\x1b[?6h\x1b[2;5H\x1b[?6lY");
    assert_eq!(terminal.cell(0, 0), 'Y');
    assert_eq!(terminal.cell(3, 4), ' ');
    assert_eq!(terminal.cell(7, 0), 'Z');
    terminal.at(0, 1);
}

#[test]
fn reset_home_clears_pending_wrap() {
    let mut terminal = Terminal::new();
    terminal.feed(b"\x1b[3;6r\x1b[?6h\x1b[2;12HA");
    assert!(terminal.term.grid().cursor.input_needs_wrap);
    terminal.feed(b"\x1b[?6l");
    assert!(!terminal.term.grid().cursor.input_needs_wrap);
    terminal.feed(b"B");
    assert_eq!(terminal.cell(0, 0), 'B');
    terminal.at(0, 1);
}

// F3: DECSC/DECRC round-trip saved-on origin state along with cursor state.
#[test]
fn saved_on_restores_origin_mode() {
    assert!(saved_on().origin());
}

#[test]
fn saved_on_restores_position_without_rehoming() {
    let mut terminal = saved_on();
    terminal.at(3, 4);
    terminal.feed(b"O");
    assert_eq!(terminal.cell(3, 4), 'O');
    terminal.at(3, 5);
}

#[test]
fn saved_on_restores_origin_addressing() {
    let mut terminal = saved_on();
    terminal.feed(b"\x1b[1;2H");
    terminal.at(2, 1);
    terminal.feed(b"\x1b[99;99H");
    terminal.at(5, 11);
}

// F4: DECSC/DECRC must also restore saved-off origin state.
#[test]
fn saved_off_restores_origin_mode() {
    assert!(!saved_off().origin());
}

#[test]
fn saved_off_restores_position_without_rehoming() {
    let mut terminal = saved_off();
    terminal.at(1, 4);
    terminal.feed(b"S");
    assert_eq!(terminal.cell(1, 4), 'S');
    terminal.at(1, 5);
}

#[test]
fn saved_off_restores_screen_addressing() {
    let mut terminal = saved_off();
    terminal.feed(b"\x1b[1;2H");
    terminal.at(0, 1);
    terminal.feed(b"\x1b[99;99H");
    terminal.at(7, 11);
}

// N: neighboring behavior and regression controls. These are separate from F1--F4.
#[test]
fn neighbor_default_margins_set_and_reset_home() {
    let mut terminal = Terminal::new();
    terminal.feed(b"\x1b[7;9H\x1b[?6h");
    terminal.at(0, 0);
    assert!(terminal.origin());
    terminal.feed(b"\x1b[6;8H\x1b[?6l");
    terminal.at(0, 0);
    assert!(!terminal.origin());
}

#[test]
fn neighbor_margins_and_bottom_clamping_remain_in_effect() {
    let mut terminal = Terminal::new();
    terminal.feed(b"\x1b[3;6r\x1b[?6h\x1b[99;99H");
    terminal.at(5, 11);
    terminal.feed(b"\x1b[?6l\x1b[99;99H");
    terminal.at(7, 11);
    terminal.feed(b"\x1b[?6h\x1b[1;1H");
    terminal.at(2, 0);
}

#[test]
fn neighbor_region_linefeed_preserves_outside_rows() {
    let mut terminal = Terminal::new();
    terminal.feed(b"\x1b[1;1HA\x1b[8;1HZ\x1b[3;6r\x1b[?6h");
    terminal.feed(b"\x1b[1;1HB\x1b[2;1HC\x1b[4;1HD\r\nE");
    assert_eq!(terminal.cell(0, 0), 'A');
    assert_eq!(terminal.cell(7, 0), 'Z');
    assert_eq!(terminal.cell(2, 0), 'C');
    assert_eq!(terminal.cell(4, 0), 'D');
    assert_eq!(terminal.cell(5, 0), 'E');
    terminal.at(5, 1);
}

#[test]
fn neighbor_changing_margins_homes_under_current_origin() {
    let mut terminal = Terminal::new();
    terminal.feed(b"\x1b[7;9H\x1b[3;6r");
    terminal.at(0, 0);
    terminal.feed(b"\x1b[?6h\x1b[4;7r");
    terminal.at(3, 0);
    terminal.feed(b"\x1b[r");
    terminal.at(0, 0);
}

#[test]
fn neighbor_same_mode_save_restore_preserves_rendition_and_charset() {
    for origin in [false, true] {
        let mut terminal = Terminal::new();
        terminal.feed(b"\x1b[3;6r");
        if origin {
            terminal.feed(b"\x1b[?6h");
        }
        terminal.feed(b"\x1b[2;5H\x1b[1;31m\x1b(0\x1b7");
        let saved = terminal.point();
        terminal.feed(b"\x1b[3;9H\x1b[0m\x1b(B\x1b8q");
        assert_eq!(terminal.origin(), origin);
        let cell = &terminal.term.grid()[saved];
        assert_eq!(cell.c, '\u{2500}');
        assert!(cell.flags.contains(Flags::BOLD));
        assert_eq!(cell.fg, Color::Named(NamedColor::Red));
        terminal.at(saved.line.0, saved.column.0 + 1);
    }
}

#[test]
fn neighbor_save_restore_preserves_saved_pending_wrap() {
    for origin in [false, true] {
        let mut terminal = Terminal::new();
        terminal.feed(b"\x1b[3;6r");
        if origin {
            terminal.feed(b"\x1b[?6h");
        }
        terminal.feed(b"\x1b[2;12HA\x1b7\x1b[3;1H\x1b8");
        let saved = terminal.point();
        assert!(terminal.term.grid().cursor.input_needs_wrap);
        terminal.feed(b"B");
        assert_eq!(terminal.cell(saved.line.0 + 1, 0), 'B');
        terminal.at(saved.line.0 + 1, 1);
        assert_eq!(terminal.origin(), origin);
    }
}

#[test]
fn neighbor_restore_does_not_roll_back_unrelated_modes() {
    for origin in [false, true] {
        let mut terminal = Terminal::new();
        terminal.feed(b"\x1b[3;6r");
        if origin {
            terminal.feed(b"\x1b[?6h");
        }
        terminal.feed(b"\x1b[2;5H\x1b7\x1b[?7l\x1b[?1h\x1b8");
        assert_eq!(terminal.origin(), origin);
        assert!(!terminal.term.mode().contains(TermMode::LINE_WRAP));
        assert!(terminal.term.mode().contains(TermMode::APP_CURSOR));
    }
}

#[test]
fn neighbor_unrelated_mode_changes_do_not_home() {
    let mut terminal = Terminal::new();
    terminal.feed(b"\x1b[3;6r\x1b[?6h\x1b[2;5H");
    terminal.at(3, 4);
    terminal.feed(b"\x1b[?25l\x1b[?1h\x1b[?7l\x1b[?25h\x1b[?1l\x1b[?7h");
    terminal.at(3, 4);
    assert!(terminal.origin());
}

#[test]
fn neighbor_restore_keeps_current_margins() {
    let mut terminal = Terminal::new();
    terminal.feed(b"\x1b[3;6r\x1b[?6h\x1b[2;5H\x1b7\x1b[2;7r\x1b8");
    terminal.at(3, 4);
    terminal.feed(b"\x1b[1;1H");
    terminal.at(1, 0);
    terminal.feed(b"\x1b[99;1H");
    terminal.at(6, 0);
}

#[test]
fn neighbor_latest_save_wins_in_both_directions() {
    for final_origin in [false, true] {
        let mut terminal = Terminal::new();
        terminal.feed(b"\x1b[3;6r");
        if !final_origin {
            terminal.feed(b"\x1b[?6h");
        }
        terminal.feed(b"\x1b[2;3H\x1b7");
        terminal.feed(if final_origin { b"\x1b[?6h" } else { b"\x1b[?6l" });
        terminal.feed(b"\x1b[3;7H\x1b7");
        let last_saved = terminal.point();
        terminal.feed(if final_origin { b"\x1b[?6l" } else { b"\x1b[?6h" });
        terminal.feed(b"\x1b[1;1H\x1b8");
        assert_eq!(terminal.origin(), final_origin);
        assert_eq!(terminal.point(), last_saved);
        terminal.feed(b"\x1b[1;2H\x1b8");
        assert_eq!(terminal.origin(), final_origin);
        assert_eq!(terminal.point(), last_saved);
    }
}

#[test]
fn neighbor_full_reset_discards_saved_origin_state() {
    let mut terminal = Terminal::new();
    terminal.feed(b"\x1b[3;6r\x1b[?6h\x1b[2;5H\x1b7\x1bc");
    assert!(!terminal.origin());
    terminal.at(0, 0);
    terminal.feed(b"\x1b[?6h\x1b[2;5H\x1b8");
    assert!(!terminal.origin());
    terminal.at(0, 0);
}
