"""Create two deliberate one-fault controls from the independent reference."""
from pathlib import Path

root = Path(__file__).resolve().parent
source = (root / "src/lib.rs").read_text()
needle = "        if self.end > text.len() {"
assert source.count(needle) == 1
bad_precedence = source.replace(
    needle,
    "        if self.start <= text.len() && !text.is_char_boundary(self.start) {\n"
    "            return Err(SpanError::NotCharBoundary);\n"
    "        }\n" + needle,
)
needle = "self.start <= offset && offset < self.end"
assert source.count(needle) == 1
bad_contains = source.replace(needle, "self.start <= offset && offset <= self.end")
for name, changed in [("negative-precedence", bad_precedence), ("negative-contains", bad_contains)]:
    target = root / name
    (target / "src").mkdir(parents=True, exist_ok=True)
    (target / "Cargo.toml").write_text((root / "Cargo.toml").read_text())
    (target / "src/lib.rs").write_text(changed)
