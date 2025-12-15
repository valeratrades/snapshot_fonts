use std::path::PathBuf;

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
	/// Generate the FillLevels font (251×251 = 63,001 two-bar glyphs)
	FillLevels {
		/// Output path for the TTF file
		#[arg(short, long)]
		output: PathBuf,
	},
	/// Generate the Candles font (52,417 candlestick glyphs)
	Candles {
		/// Output path for the TTF file
		#[arg(short, long)]
		output: PathBuf,
	},
}

fn main() -> Result<()> {
	color_eyre::install()?;

	let cli = Cli::parse();

	match cli.command {
		Commands::FillLevels { output } => {
			snapshot_fonts::generate_font(&output)?;
			println!("Generated FillLevels font: {}", output.display());
		}
		Commands::Candles { output } => {
			snapshot_fonts::generate_candle_font(&output)?;
			println!("Generated Candles font: {}", output.display());
		}
	}

	Ok(())
}
