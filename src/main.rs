use clap::{Parser, Subcommand};
use color_eyre::Result;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

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
    let script = format!(
        r#"
import fontforge

font = fontforge.font()
font.fontname = "FillLevels"
font.familyname = "FillLevels"
font.fullname = "FillLevels Regular"
font.encoding = "UnicodeBMP"
font.em = 1024
font.ascent = 1024
font.descent = 0

for i in range(256):
    glyph = font.createChar(i)
    glyph.width = 1024
    fill_height = 0 if i == 0 else int((i / 255.0) * 1024)
    if fill_height > 0:
        pen = glyph.glyphPen()
        pen.moveTo((0, 0))
        pen.lineTo((1024, 0))
        pen.lineTo((1024, fill_height))
        pen.lineTo((0, fill_height))
        pen.closePath()

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
