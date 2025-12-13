use std::{
	io::Write,
	path::PathBuf,
	process::{Command, Stdio},
};

use clap::{Parser, Subcommand};
use color_eyre::Result;

#[derive(Parser)]
#[command(name = "snapshot_fonts")]
#[command(about = "Generate special-purpose fonts")]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Generate a font with 256 fill-level bar glyphs (chars 0-255)
	Bars {
		/// Output path for the TTF file
		#[arg(short, long)]
		output: PathBuf,
	},
}

fn main() -> Result<()> {
	color_eyre::install()?;

	let cli = Cli::parse();

	match cli.command {
		Commands::Bars { output } => generate_bars_font(&output)?,
	}

	Ok(())
}

fn generate_bars_font(output: &PathBuf) -> Result<()> {
	// 251 × 251 = 63,001 glyphs (two bars per char, each 0-250 height)
	// Char code = left * 251 + right, skipping surrogate range 0xD800-0xDFFF
	let script = format!(
		r#"
import fontforge

font = fontforge.font()
font.fontname = "FillLevels"
font.familyname = "FillLevels"
font.fullname = "FillLevels Regular"
font.encoding = "UnicodeFull"
font.em = 1024
font.ascent = 1024
font.descent = 0

LEVELS = 251
SURROGATE_START = 0xD800
SURROGATE_LEN = 2048

for left in range(LEVELS):
    for right in range(LEVELS):
        char_code = left * LEVELS + right
        if char_code >= SURROGATE_START:
            char_code += SURROGATE_LEN
        glyph = font.createChar(char_code)
        glyph.width = 1024

        left_height = int((left / 250.0) * 1024)
        right_height = int((right / 250.0) * 1024)

        pen = glyph.glyphPen()
        if left_height > 0:
            pen.moveTo((0, 0))
            pen.lineTo((512, 0))
            pen.lineTo((512, left_height))
            pen.lineTo((0, left_height))
            pen.closePath()

        if right_height > 0:
            pen.moveTo((512, 0))
            pen.lineTo((1024, 0))
            pen.lineTo((1024, right_height))
            pen.lineTo((512, right_height))
            pen.closePath()
        pen = None

font.generate("{}")
"#,
		output.display()
	);

	let mut child = Command::new("nix-shell")
		.args(["-p", "fontforge", "--run", "fontforge -lang=py -script /dev/stdin"])
		.stdin(Stdio::piped())
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()?;

	child.stdin.take().unwrap().write_all(script.as_bytes())?;

	let status = child.wait()?;
	if status.success() {
		println!("Generated {}", output.display());
	} else {
		color_eyre::eyre::bail!("fontforge failed");
	}

	Ok(())
}
