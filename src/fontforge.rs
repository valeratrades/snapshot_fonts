use std::{
	io::Write,
	path::Path,
	process::{Command, Stdio},
};

// Font metric constants (matching DejaVu Sans Mono)
pub const HHEA_ASCENT: i32 = 1901;
pub const HHEA_DESCENT: i32 = 483;
pub const LINE_HEIGHT: i32 = HHEA_ASCENT + HHEA_DESCENT; // 2384
pub const GLYPH_WIDTH: i32 = 1233; // DejaVu Sans Mono width (1233 in 2048 em = 0.602 ratio)

/// Common Python fontforge setup for DejaVu Sans Mono-compatible fonts
pub const FONT_SETUP: &str = r#"
import fontforge

# Match DejaVu Sans Mono metrics exactly
# hhea: ascent=1901, descent=-483 (line height = 2384)
HHEA_ASCENT = 1901
HHEA_DESCENT = 483
LINE_HEIGHT = HHEA_ASCENT + HHEA_DESCENT  # 2384
GLYPH_WIDTH = 1233  # DejaVu Sans Mono width (1233 in 2048 em = 0.602 ratio)

def create_font(name):
    font = fontforge.font()
    font.fontname = name
    font.familyname = name
    font.fullname = f"{name} Regular"
    font.encoding = "UnicodeFull"

    font.em = 2048
    font.ascent = HHEA_ASCENT
    font.descent = 2048 - HHEA_ASCENT

    font.hhea_ascent = HHEA_ASCENT
    font.hhea_descent = -HHEA_DESCENT
    font.hhea_linegap = 0
    font.hhea_ascent_add = 0
    font.hhea_descent_add = 0

    font.os2_typoascent = HHEA_ASCENT
    font.os2_typodescent = -HHEA_DESCENT
    font.os2_typolinegap = 0
    font.os2_typoascent_add = 0
    font.os2_typodescent_add = 0
    font.os2_winascent = HHEA_ASCENT
    font.os2_windescent = HHEA_DESCENT
    font.os2_winascent_add = 0
    font.os2_windescent_add = 0
    font.os2_use_typo_metrics = False

    # Mark as monospace font
    font.os2_panose = (2, 11, 5, 9, 2, 2, 3, 2, 2, 7)
    font.is_quadratic = True

    return font
"#;

/// Run a fontforge Python script
pub fn run_fontforge_script(script: &str) -> std::io::Result<()> {
	let mut child = Command::new("fontforge")
		.args(["-lang=py", "-script", "/dev/stdin"])
		.stdin(Stdio::piped())
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
		.or_else(|_| {
			Command::new("nix-shell")
				.args(["-p", "fontforge", "--run", "fontforge -lang=py -script /dev/stdin"])
				.stdin(Stdio::piped())
				.stdout(Stdio::null())
				.stderr(Stdio::null())
				.spawn()
		})?;

	child.stdin.take().unwrap().write_all(script.as_bytes())?;

	let status = child.wait()?;
	if status.success() { Ok(()) } else { Err(std::io::Error::other("fontforge failed")) }
}

/// Generate a font file using a custom glyph generation script
pub fn generate_font(name: &str, output: &Path, glyph_script: &str) -> std::io::Result<()> {
	let script = format!(
		r#"{setup}
font = create_font("{name}")

{glyph_script}

font.generate("{output}")
"#,
		setup = FONT_SETUP,
		name = name,
		glyph_script = glyph_script,
		output = output.display()
	);

	run_fontforge_script(&script)
}
