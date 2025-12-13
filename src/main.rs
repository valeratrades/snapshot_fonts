use clap::{Parser, Subcommand};
use color_eyre::Result;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use v_utils::xdg_cache_file;

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
    let bdf_path = xdg_cache_file!("bars.bdf");

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

# Create 256 glyphs for chars 0-255
for i in range(256):
    glyph = font.createChar(i)
    glyph.width = 1024

    # Fill height proportional to char code
    if i == 0:
        fill_height = 0
    else:
        fill_height = int((i / 255.0) * 1024)

    if fill_height > 0:
        # Draw a rectangle from (0,0) to (1024, fill_height)
        pen = glyph.glyphPen()
        pen.moveTo((0, 0))
        pen.lineTo((1024, 0))
        pen.lineTo((1024, fill_height))
        pen.lineTo((0, fill_height))
        pen.closePath()
        pen = None

font.generate("{}")
print("Generated {} with 256 glyphs")
"#,
        output.display(),
        output.display()
    );

    // Write script to cache location
    fs::create_dir_all(bdf_path.parent().unwrap())?;
    let script_path = bdf_path.with_extension("py");
    fs::write(&script_path, &script)?;

    // Run fontforge
    let status = Command::new("nix-shell")
        .args([
            "-p",
            "fontforge",
            "--run",
            &format!("fontforge -lang=py -script {}", script_path.display()),
        ])
        .status()?;

    // Clean up
    fs::remove_file(&script_path).ok();

    if status.success() {
        println!("Generated {}", output.display());
    } else {
        color_eyre::eyre::bail!("fontforge failed");
    }

    Ok(())
}
