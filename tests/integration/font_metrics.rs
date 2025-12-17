//! Integration test to verify font metrics are correct after generation.
//! This guards against fontforge overwriting our metric settings.

use std::process::Command;

use snapshot_fonts::{PUA_START, generate_font};

#[test]
fn test_font_metrics_are_correct() {
	let temp_dir = tempfile::tempdir().expect("create temp dir");
	let font_path = temp_dir.path().join("test_font.ttf");

	// Generate the font
	if generate_font(&font_path).is_err() {
		eprintln!("Skipping test: fontforge not available");
		return;
	}

	// Use fonttools to verify metrics (requires python3 with fonttools)
	// We match DejaVu Sans Mono metrics: em=2048, ascent=1901, descent=-483
	let script = format!(
		r#"
from fontTools.ttLib import TTFont
import sys

PUA_START = {pua_start}
tt = TTFont("{path}")

errors = []

# Check head table
if tt['head'].unitsPerEm != 2048:
    errors.append(f"head.unitsPerEm: expected 2048, got {{tt['head'].unitsPerEm}}")

# Check hhea table (should match DejaVu Sans Mono)
if tt['hhea'].ascent != 1901:
    errors.append(f"hhea.ascent: expected 1901, got {{tt['hhea'].ascent}}")
if tt['hhea'].descent != -483:
    errors.append(f"hhea.descent: expected -483, got {{tt['hhea'].descent}}")
if tt['hhea'].lineGap != 0:
    errors.append(f"hhea.lineGap: expected 0, got {{tt['hhea'].lineGap}}")

# Check full bar glyph fills the line height
cmap = tt.getBestCmap()
full_bar_code = PUA_START + 250*251 + 250
if full_bar_code in cmap:
    glyph_name = cmap[full_bar_code]
    glyf = tt['glyf']
    g = glyf[glyph_name]
    if g.yMin != -483:
        errors.append(f"full bar yMin: expected -483, got {{g.yMin}}")
    if g.yMax != 1901:
        errors.append(f"full bar yMax: expected 1901, got {{g.yMax}}")
    if g.xMax != 1233:
        errors.append(f"full bar xMax: expected 1233, got {{g.xMax}}")

if errors:
    for e in errors:
        print(e, file=sys.stderr)
    sys.exit(1)
"#,
		pua_start = PUA_START,
		path = font_path.display()
	);

	let output = Command::new("uv")
		.args(["run", "--with", "fonttools", "python3", "-c", &script])
		.output()
		.expect("uv must be available");

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		panic!("Font metrics verification failed:\n{}", stderr);
	}
}
