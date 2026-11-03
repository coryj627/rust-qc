Correct origin mode (DECOM) in `alacritty_terminal` 0.25.0.

The source workspace is `/work/crate`. Only the `alacritty_terminal` crate is in scope. The reported behavior comes from Alacritty issue #8636: enabling origin mode leaves the cursor at its previous screen position, which can allow subsequent text to be written outside the scrolling region. Saving and restoring the cursor also fails to restore the saved origin-mode state.

Implement these behavioral requirements:

- Processing DECOM set (`CSI ? 6 h`) homes the cursor using the newly enabled origin: the first column of the top row of the scrolling region.
- Processing DECOM reset (`CSI ? 6 l`) homes the cursor using the newly disabled origin: the first column of the screen. Both operations must leave subsequent printable text at the appropriate home position, including when the previous cursor had reached the last column.
- DECSC (`ESC 7`) and DECRC (`ESC 8`) save and restore origin mode along with the established cursor state. This must work for both saved-on and saved-off origin mode, even if DECOM changes between saving and restoring. Restore the saved cursor position; subsequent addressing must use the restored origin mode.
- Preserve unrelated terminal behavior, public interfaces, and established project conventions. Existing cursor rendition, character-set and wrap behavior, scrolling margins, and unrelated modes should retain their established behavior.

Add focused regression coverage using non-default scrolling margins and neighboring cases, as well as the two saved origin states. Prefer behavior exercised through terminal escape-sequence processing. Do not limit verification to the full-screen default scrolling region.

Build and test only `alacritty_terminal`, never the graphical application or the whole workspace. Dependencies are already fetched. Use `cargo --locked --offline`, an installed Rust toolchain, `RUSTUP_AUTO_INSTALL=0`, and `/work/tmp` for executable temporary files. A terminal-only test command is:

```sh
cd /work/crate
RUSTUP_AUTO_INSTALL=0 TMPDIR=/work/tmp cargo +1.99.1 test --locked --offline -p alacritty_terminal
```

Do not fetch external material or install tools. Finish with a concise explanation of the change and the validation actually performed, including any remaining limitations.
