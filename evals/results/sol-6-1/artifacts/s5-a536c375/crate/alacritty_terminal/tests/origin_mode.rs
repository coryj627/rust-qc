use alacritty_terminal::event::VoidListener;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::{Color, NamedColor, Processor};

fn advance(term: &mut Term<VoidListener>, input: &str) {
    let mut parser: Processor = Processor::new();
    for byte in input.bytes() {
        parser.advance(term, byte);
    }
}

fn terminal() -> Term<VoidListener> {
    let mut term = Term::new(Config::default(), &TermSize::new(6, 8), VoidListener);
    advance(&mut term, "\x1b[3;6r");
    term
}

// Regression coverage for issue #8636.
#[test]
fn origin_mode_set_homes_cursor() {
    for (case, position) in [
        ("above margins", "\x1b[1;4H"),
        ("inside margins", "\x1b[4;4H"),
        ("below margins", "\x1b[8;4H"),
        ("pending wrap above margins", "\x1b[1;6Hx"),
        ("pending wrap below margins", "\x1b[8;6Hx"),
        ("already enabled with pending wrap", "\x1b[?6h\x1b[2;6Hx"),
    ] {
        let mut term = terminal();
        advance(&mut term, position);
        advance(&mut term, "\x1b[?6h");

        assert_eq!(term.grid().cursor.point, Point::new(Line(2), Column(0)), "{case}");
        advance(&mut term, "A");
        assert_eq!(term.grid()[Line(2)][Column(0)].c, 'A', "{case}");
        assert_eq!(term.grid().cursor.point, Point::new(Line(2), Column(1)), "{case}");
    }
}

#[test]
fn origin_mode_reset_homes_cursor() {
    for (case, position) in [
        ("inside margins", "\x1b[?6h\x1b[2;4H"),
        ("pending wrap at top margin", "\x1b[?6h\x1b[1;6Hx"),
        ("pending wrap at bottom margin", "\x1b[?6h\x1b[4;6Hx"),
        ("already disabled with pending wrap", "\x1b[8;6Hx"),
    ] {
        let mut term = terminal();
        advance(&mut term, position);
        advance(&mut term, "\x1b[?6l");

        assert_eq!(term.grid().cursor.point, Point::new(Line(0), Column(0)), "{case}");
        advance(&mut term, "A");
        assert_eq!(term.grid()[Line(0)][Column(0)].c, 'A', "{case}");
        assert_eq!(term.grid().cursor.point, Point::new(Line(0), Column(1)), "{case}");
    }
}

#[test]
fn restore_cursor_restores_origin_mode_on() {
    let mut term = terminal();
    advance(&mut term, "\x1b[?6h\x1b[2;3H\x1b7\x1b[?6l\x1b[8;5H\x1b8");

    assert_eq!(term.grid().cursor.point, Point::new(Line(3), Column(2)));
    assert!(term.mode().contains(TermMode::ORIGIN));
    advance(&mut term, "A\x1b[2;2HB");
    assert_eq!(term.grid()[Line(3)][Column(2)].c, 'A');
    assert_eq!(term.grid()[Line(3)][Column(1)].c, 'B');

    // Addressing remains limited by the original scrolling margins.
    advance(&mut term, "\x1b[99;2H");
    assert_eq!(term.grid().cursor.point, Point::new(Line(5), Column(1)));
}

#[test]
fn restore_cursor_restores_origin_mode_off() {
    let mut term = terminal();
    advance(&mut term, "\x1b[2;3H\x1b7\x1b[?6h\x1b[3;5H\x1b8");

    // A saved position above the scrolling region must also be restored exactly.
    assert_eq!(term.grid().cursor.point, Point::new(Line(1), Column(2)));
    assert!(!term.mode().contains(TermMode::ORIGIN));
    advance(&mut term, "A\x1b[2;2HB");
    assert_eq!(term.grid()[Line(1)][Column(2)].c, 'A');
    assert_eq!(term.grid()[Line(1)][Column(1)].c, 'B');

    advance(&mut term, "\x1b[99;2H");
    assert_eq!(term.grid().cursor.point, Point::new(Line(7), Column(1)));
}

#[test]
fn restore_origin_preserves_cursor_state_and_unrelated_modes() {
    let mut term = terminal();
    advance(&mut term, "\x1b[?6h\x1b[2;6H\x1b[1;31m\x1b(0q\x1b7");
    advance(&mut term, "\x1b[?6l\x1b[0m\x1b(B\x1b[?7l\x1b[?1h\x1b8");

    assert_eq!(term.grid().cursor.point, Point::new(Line(3), Column(5)));
    assert!(term.grid().cursor.input_needs_wrap);
    assert!(term.mode().contains(TermMode::ORIGIN | TermMode::APP_CURSOR));
    assert!(!term.mode().contains(TermMode::LINE_WRAP));

    advance(&mut term, "\x1b[?7hq");
    let cell = &term.grid()[Line(4)][Column(0)];
    assert_eq!(cell.c, '─');
    assert_eq!(cell.fg, Color::Named(NamedColor::Red));
    assert!(cell.flags.contains(Flags::BOLD));
    assert!(term.grid()[Line(3)][Column(5)].flags.contains(Flags::WRAPLINE));
}

#[test]
fn origin_mode_is_saved_separately_on_each_screen() {
    let mut term = terminal();
    advance(&mut term, "\x1b[?6h\x1b[2;3H\x1b[?1049h");
    advance(&mut term, "\x1b[?6l\x1b[1;5H\x1b7\x1b[?1049l\x1b8");

    assert_eq!(term.grid().cursor.point, Point::new(Line(3), Column(2)));
    assert!(term.mode().contains(TermMode::ORIGIN));

    advance(&mut term, "\x1b[?1049h\x1b8");
    assert_eq!(term.grid().cursor.point, Point::new(Line(0), Column(4)));
    assert!(!term.mode().contains(TermMode::ORIGIN));
}

#[test]
fn terminal_reset_clears_saved_origin_mode() {
    let mut term = terminal();
    advance(&mut term, "\x1b[?6h\x1b7\x1bc\x1b8");

    assert_eq!(term.grid().cursor.point, Point::new(Line(0), Column(0)));
    assert!(!term.mode().contains(TermMode::ORIGIN));
    advance(&mut term, "\x1b[3;6r\x1b[1;1HA");
    assert_eq!(term.grid()[Line(0)][Column(0)].c, 'A');
}
