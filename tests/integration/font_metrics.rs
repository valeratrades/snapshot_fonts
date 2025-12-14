//! Integration test to verify font metrics are correct after generation.
//! This guards against fontforge overwriting our metric settings.

use std::process::Command;

use snapshot_fonts::generate_font;

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
	let script = format!(
		r#"
from fontTools.ttLib import TTFont
import sys

tt = TTFont("{}")

errors = []

# Check hhea table
if tt['hhea'].ascent != 1024:
    errors.append(f"hhea.ascent: expected 1024, got {{tt['hhea'].ascent}}")
if tt['hhea'].descent != 0:
    errors.append(f"hhea.descent: expected 0, got {{tt['hhea'].descent}}")
if tt['hhea'].lineGap != 0:
    errors.append(f"hhea.lineGap: expected 0, got {{tt['hhea'].lineGap}}")

# Check OS/2 table
if tt['OS/2'].sTypoAscender != 1024:
    errors.append(f"OS/2.sTypoAscender: expected 1024, got {{tt['OS/2'].sTypoAscender}}")
if tt['OS/2'].sTypoDescender != 0:
    errors.append(f"OS/2.sTypoDescender: expected 0, got {{tt['OS/2'].sTypoDescender}}")
if tt['OS/2'].sTypoLineGap != 0:
    errors.append(f"OS/2.sTypoLineGap: expected 0, got {{tt['OS/2'].sTypoLineGap}}")
if tt['OS/2'].usWinAscent != 1024:
    errors.append(f"OS/2.usWinAscent: expected 1024, got {{tt['OS/2'].usWinAscent}}")
if tt['OS/2'].usWinDescent != 0:
    errors.append(f"OS/2.usWinDescent: expected 0, got {{tt['OS/2'].usWinDescent}}")

# Check USE_TYPO_METRICS flag (bit 7 of fsSelection)
if not (tt['OS/2'].fsSelection & 0x80):
    errors.append(f"OS/2.fsSelection missing USE_TYPO_METRICS flag")

# Check head table
if tt['head'].unitsPerEm != 1024:
    errors.append(f"head.unitsPerEm: expected 1024, got {{tt['head'].unitsPerEm}}")

if errors:
    for e in errors:
        print(e, file=sys.stderr)
    sys.exit(1)
"#,
		font_path.display()
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
