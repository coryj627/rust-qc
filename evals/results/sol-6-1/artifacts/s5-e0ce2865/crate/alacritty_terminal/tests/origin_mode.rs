//! Regression tests for origin mode (https://github.com/alacritty/alacritty/issues/8636).

use alacritty_terminal::event::VoidListener;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::{self, Color, NamedColor};

fn advance(term: &mut Term<VoidListener>, input: &[u8]) {
    let mut parser: ansi::Processor = ansi::Processor::new();
    for &byte in input {
        parser.advance(term, byte);
    }
}

#[test]
fn origin_mode_set_homes_cursor() {
    for position in [
        &b"\x1b[1;4HX"[..],
        &b"\x1b[4;4HX"[..],
        &b"\x1b[7;4HX"[..],
        &b"\x1b[7;8HX"[..],
        &b"\x1b[?6h\x1b[2;8HX"[..],
    ] {
        let size = TermSize::new(8, 7);
        let mut term = Term::new(Config::default(), &size, VoidListener);
        advance(&mut term, b"\x1b[3;5r");
        advance(&mut term, position);

        advance(&mut term, b"\x1b[?6h");
        assert_eq!(term.grid().cursor.point, Point::new(Line(2), Column(0)));
        assert!(term.mode().contains(TermMode::ORIGIN));

        advance(&mut term, b"A");
        assert_eq!(term.grid()[Line(2)][Column(0)].c, 'A');
        assert_eq!(term.grid().cursor.point, Point::new(Line(2), Column(1)));

        // Addressing is relative to the unchanged scrolling region and clamped at its bottom.
        advance(&mut term, b"\x1b[99;1HB");
        assert_eq!(term.grid()[Line(4)][Column(0)].c, 'B');
    }
}

#[test]
fn origin_mode_reset_homes_cursor() {
    for position in [
        &b"\x1b[?6h\x1b[1;4HX"[..],
        &b"\x1b[?6h\x1b[2;4HX"[..],
        &b"\x1b[?6h\x1b[3;4HX"[..],
        &b"\x1b[?6h\x1b[3;8HX"[..],
        &b"\x1b[7;8HX"[..],
    ] {
        let size = TermSize::new(8, 7);
        let mut term = Term::new(Config::default(), &size, VoidListener);
        advance(&mut term, b"\x1b[3;5r");
        advance(&mut term, position);

        advance(&mut term, b"\x1b[?6l");
        assert_eq!(term.grid().cursor.point, Point::new(Line(0), Column(0)));
        assert!(!term.mode().contains(TermMode::ORIGIN));

        advance(&mut term, b"A");
        assert_eq!(term.grid()[Line(0)][Column(0)].c, 'A');
        assert_eq!(term.grid().cursor.point, Point::new(Line(0), Column(1)));

        advance(&mut term, b"\x1b[99;1HB");
        assert_eq!(term.grid()[Line(6)][Column(0)].c, 'B');
    }
}

#[test]
fn saved_origin_mode_on_is_restored() {
    let size = TermSize::new(8, 7);
    let mut term = Term::new(Config::default(), &size, VoidListener);
    advance(&mut term, b"\x1b[3;5r\x1b[?6h\x1b[2;4H\x1b[1;31m\x1b(0\x1b7");
    advance(&mut term, b"\x1b[?6l\x1b[7;8H\x1b[0m\x1b(B\x1b[?7l\x1b[?2004h");

    advance(&mut term, b"\x1b8");
    assert_eq!(term.grid().cursor.point, Point::new(Line(3), Column(3)));
    assert!(term.mode().contains(TermMode::ORIGIN));
    assert!(!term.mode().contains(TermMode::LINE_WRAP));
    assert!(term.mode().contains(TermMode::BRACKETED_PASTE));

    advance(&mut term, b"q");
    let cell = &term.grid()[Line(3)][Column(3)];
    assert_eq!(cell.c, '─');
    assert_eq!(cell.fg, Color::Named(NamedColor::Red));
    assert!(cell.flags.contains(Flags::BOLD));

    advance(&mut term, b"\x1b[1;2HA\x1b[99;1HB");
    assert_eq!(term.grid()[Line(2)][Column(1)].c, 'A');
    assert_eq!(term.grid()[Line(4)][Column(0)].c, 'B');
}

#[test]
fn saved_origin_mode_off_is_restored() {
    let size = TermSize::new(8, 7);
    let mut term = Term::new(Config::default(), &size, VoidListener);
    // Save outside the scrolling region, so restore must not clamp or offset the saved position.
    advance(&mut term, b"\x1b[3;5r\x1b[7;4H\x1b[?7l\x1b7");
    advance(&mut term, b"\x1b[?6h\x1b[2;8H\x1b[?7h\x1b[?2004h");

    advance(&mut term, b"\x1b8");
    assert_eq!(term.grid().cursor.point, Point::new(Line(6), Column(3)));
    assert!(!term.mode().contains(TermMode::ORIGIN));
    assert!(term.mode().contains(TermMode::LINE_WRAP));
    assert!(term.mode().contains(TermMode::BRACKETED_PASTE));

    advance(&mut term, b"A\x1b[1;2HB");
    assert_eq!(term.grid()[Line(6)][Column(3)].c, 'A');
    assert_eq!(term.grid()[Line(0)][Column(1)].c, 'B');

    // Restoring the origin flag must not reset the scrolling margins.
    advance(&mut term, b"\x1b[?6hC");
    assert_eq!(term.grid()[Line(2)][Column(0)].c, 'C');
}

#[test]
fn saved_origin_mode_preserves_pending_wrap() {
    for (save, change) in [
        (&b"\x1b[?6h\x1b[2;8HX\x1b7"[..], &b"\x1b[?6l"[..]),
        (&b"\x1b[4;8HX\x1b7"[..], &b"\x1b[?6h"[..]),
    ] {
        let size = TermSize::new(8, 7);
        let mut term = Term::new(Config::default(), &size, VoidListener);
        advance(&mut term, b"\x1b[3;5r");
        advance(&mut term, save);
        advance(&mut term, change);

        advance(&mut term, b"\x1b8Y");
        assert_eq!(term.grid()[Line(3)][Column(7)].c, 'X');
        assert!(term.grid()[Line(3)][Column(7)].flags.contains(Flags::WRAPLINE));
        assert_eq!(term.grid()[Line(4)][Column(0)].c, 'Y');
        assert_eq!(term.grid().cursor.point, Point::new(Line(4), Column(1)));
    }
}

#[test]
fn saved_origin_mode_follows_screen_buffer() {
    let size = TermSize::new(8, 7);
    let mut term = Term::new(Config::default(), &size, VoidListener);
    advance(&mut term, b"\x1b[3;5r\x1b[?6h\x1b[2;3H\x1b[?1049h");
    advance(&mut term, b"\x1b[?6l\x1b[7;4H\x1b7\x1b[?6h\x1b8");
    assert_eq!(term.grid().cursor.point, Point::new(Line(6), Column(3)));
    assert!(!term.mode().contains(TermMode::ORIGIN));

    advance(&mut term, b"\x1b[?1049l\x1b8");
    assert_eq!(term.grid().cursor.point, Point::new(Line(3), Column(2)));
    assert!(term.mode().contains(TermMode::ORIGIN));
    advance(&mut term, b"\x1b[1;1HA");
    assert_eq!(term.grid()[Line(2)][Column(0)].c, 'A');
}

#[test]
fn reset_clears_saved_origin_mode() {
    let size = TermSize::new(8, 7);
    let mut term = Term::new(Config::default(), &size, VoidListener);
    advance(&mut term, b"\x1b[3;5r\x1b[?6h\x1b7\x1b[?1049h\x1b7\x1bc");

    advance(&mut term, b"\x1b[3;5r\x1b[?6h\x1b8");
    assert_eq!(term.grid().cursor.point, Point::new(Line(0), Column(0)));
    assert!(!term.mode().contains(TermMode::ORIGIN));

    advance(&mut term, b"\x1b[?1049h\x1b[?6h\x1b8");
    assert_eq!(term.grid().cursor.point, Point::new(Line(0), Column(0)));
    assert!(!term.mode().contains(TermMode::ORIGIN));
}
