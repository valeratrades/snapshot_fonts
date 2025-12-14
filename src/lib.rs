use std::{
	io::Write,
	path::Path,
	process::{Command, Stdio},
	sync::OnceLock,
};

pub const LEVELS: u16 = 251;
/// Start of Private Use Area - Plane 15 (PUA-A)
pub const PUA_START: u32 = 0xf0000;

/// Standard Unicode block characters for fallback mode (8 levels)
const FALLBACK_BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Check if FillLevels font is available on the system
fn font_available() -> bool {
	static AVAILABLE: OnceLock<bool> = OnceLock::new();
	*AVAILABLE.get_or_init(|| {
		Command::new("fc-list")
			.args([":", "family"])
			.output()
			.map(|o| String::from_utf8_lossy(&o.stdout).contains("FillLevels"))
			.unwrap_or(false)
	})
}

/// Encode two bar values (0-250 each) into a Unicode codepoint in PUA-A (Plane 15)
pub fn encode_bars(left: u8, right: u8) -> char {
	debug_assert!(left <= 250, "left must be 0-250");
	debug_assert!(right <= 250, "right must be 0-250");

	let code = PUA_START + left as u32 * LEVELS as u32 + right as u32;
	char::from_u32(code).expect("valid codepoint")
}

/// Decode a Unicode codepoint back to two bar values
pub fn decode_bars(c: char) -> (u8, u8) {
	let code = c as u32 - PUA_START;
	let left = (code / LEVELS as u32) as u8;
	let right = (code % LEVELS as u32) as u8;
	(left, right)
}

/// Generate the FillLevels TTF font file
pub fn generate_font(output: &Path) -> std::io::Result<()> {
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

LEVELS = {levels}
PUA_START = {pua_start}

for left in range(LEVELS):
    for right in range(LEVELS):
        char_code = PUA_START + left * LEVELS + right
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

font.generate("{output}")
"#,
		levels = LEVELS,
		pua_start = PUA_START,
		output = output.display()
	);

	// Try fontforge directly first (works in nix build), fall back to nix-shell (works in dev)
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

// ============================================================================
// Snapshot plotting - uses FillLevels font (251 levels) or fallback (8 levels)
// ============================================================================

static SINGLE_PLOT_WIDTH: usize = 90;
static SINGLE_PLOT_HEIGHT: usize = 12;

struct PlotData {
	scale: f64,
	offset: f64,
	levels_per_row: usize,
	use_font: bool,
}

impl PlotData {
	fn new(min_val: f64, max_val: f64, height: usize, use_font: bool) -> Self {
		// Font: 251 levels (0-250), Fallback: 9 levels (0-8)
		let levels_per_row = if use_font { 251 } else { 9 };
		let data_range = max_val - min_val;
		let plot_range = (height * levels_per_row) as f64;
		let scale = plot_range / data_range;
		let offset = min_val * scale;
		PlotData {
			scale,
			offset,
			levels_per_row,
			use_font,
		}
	}

	fn get_level(&self, val: f64, row: usize) -> u8 {
		let scaled_val = val * self.scale - self.offset;
		// Max index: 250 for font, 8 for fallback
		let max_level = (self.levels_per_row - 1) as f64;
		(scaled_val - row as f64 * self.levels_per_row as f64).clamp(0.0, max_level) as u8
	}

	fn level_to_char(&self, level: u8) -> char {
		if self.use_font { encode_bars(level, level) } else { FALLBACK_BLOCKS[level as usize] }
	}

	fn empty_char(&self) -> char {
		if self.use_font { encode_bars(0, 0) } else { ' ' }
	}

	/// Raise by the smallest step
	fn raise_plot(&mut self) {
		self.offset -= 1.0;
	}
}

#[derive(Clone, Debug, Default)]
pub struct SnapshotP {
	prices: Vec<f64>,
	secondary_pane: Option<Vec<Option<f64>>>,
	width: usize,
	height: usize,
}

impl SnapshotP {
	pub fn build<T: Into<f64> + Copy>(prices: &[T]) -> Self {
		SnapshotP {
			prices: prices.iter().map(|x| (*x).into()).collect(),
			secondary_pane: None,
			width: SINGLE_PLOT_WIDTH,
			height: SINGLE_PLOT_HEIGHT,
		}
	}

	/// Height is always 2/5 that of the main pane
	pub fn secondary_pane_optional<T: Into<f64> + Copy>(self, secondary_pane: Vec<Option<T>>) -> Self {
		SnapshotP {
			secondary_pane: Some(secondary_pane.iter().map(|x| x.map(|x| x.into())).collect()),
			..self
		}
	}

	/// Height is always 2/5 that of the main pane
	pub fn secondary_pane<T: Into<f64> + Copy>(self, secondary_pane: Vec<T>) -> Self {
		SnapshotP {
			secondary_pane: Some(secondary_pane.iter().map(|x| Some((*x).into())).collect()),
			..self
		}
	}

	/// Default width is `90`
	pub fn width(self, width: usize) -> Self {
		SnapshotP { width, ..self }
	}

	/// Set height of the main pane. Secondary pane's height is automatically determined. Default height is `12`
	pub fn height_main_pane(self, height: usize) -> Self {
		SnapshotP { height, ..self }
	}

	/// # Panics
	/// Meant to be used only in tests, so if any input params are incorrect we panic.
	pub fn draw(self) -> String {
		let use_font = font_available();
		let header = if use_font {
			String::new()
		} else {
			"# fallback (no FillLevels.ttf font located)\n".to_string()
		};
		let main_section = Self::plot_p(self.prices, self.width, self.height, use_font);
		let mut out = format!("{header}{main_section}");
		if let Some(secondary_pane) = self.secondary_pane {
			let separator = "─".repeat(self.width);
			let secondary_section = Self::plot_p_optional(secondary_pane, self.width, (self.height * 3) / 5, use_font);
			out.push_str(&format!("\n{separator}\n{secondary_section}"));
		}
		out
	}

	fn plot_p_optional(prices: Vec<Option<f64>>, width: usize, height: usize, use_font: bool) -> String {
		let empty_char = if use_font { encode_bars(0, 0) } else { ' ' };
		if prices.is_empty() {
			return (0..height).map(|_| empty_char.to_string().repeat(width)).collect::<Vec<_>>().join("\n");
		}
		let non_empty_prices = prices.iter().filter_map(|x| *x).collect::<Vec<f64>>();

		let min_val = non_empty_prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
		let max_val = non_empty_prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

		let min_step = (max_val - min_val) / 100.0;
		let f_len = min_step.to_string().split('.').collect::<Vec<&str>>()[1].chars().take_while(|&c| c == '0').count() + 1;
		let max_str = format!("{:.f_len$}", max_val).trim_end_matches(".0").to_string();
		let min_str = format!("{:.f_len$}", min_val).trim_end_matches(".0").to_string();
		let side_panel_width = max_str.len().max(min_str.len());
		let mut side_panel = String::with_capacity(height * side_panel_width);
		for i in 0..height {
			if i == 0 {
				side_panel.push_str(&format!("{max_str}\n"));
			} else if i == height - 1 {
				side_panel.push_str(&format!("{min_str}\n"));
			} else {
				side_panel.push_str(&format!("{:>side_panel_width$}\n", " "));
			}
		}
		side_panel.pop(); // remove last newline

		if (max_val - min_val).abs() < f64::EPSILON {
			return (0..height).map(|_| empty_char.to_string().repeat(width)).collect::<Vec<_>>().join("\n");
		}

		let mut plot_data = PlotData::new(min_val, max_val, height, use_font);
		plot_data.raise_plot(); // here we always want to raise to be able to distinguish between empty and non-empty prices

		let mut plot = Vec::with_capacity(height);
		for row in (0..height).rev() {
			let row_str: String = (0..width)
				.map(|j| {
					let index = (j as f64 * prices.len() as f64 / width as f64) as usize;
					match prices[index] {
						Some(val) => {
							let level = plot_data.get_level(val, row);
							plot_data.level_to_char(level)
						}
						None => plot_data.empty_char(),
					}
				})
				.collect();
			plot.push(row_str);
		}

		join_str_blocks_v(plot.join("\n"), side_panel)
	}

	fn plot_p(prices: Vec<f64>, width: usize, height: usize, use_font: bool) -> String {
		if prices.is_empty() {
			panic!("prices are empty");
		}

		let min_val = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
		let max_val = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
		let mid_char = if use_font { encode_bars(125, 125) } else { '▄' };
		if (max_val - min_val).abs() < f64::EPSILON {
			return (0..height).map(|_| mid_char.to_string().repeat(width)).collect::<Vec<_>>().join("\n");
		}

		let min_step = (max_val - min_val) / 100.0;
		let f_len = min_step.to_string().split('.').collect::<Vec<&str>>()[1].chars().take_while(|&c| c == '0').count() + 1;
		let max_str = format!("{:.f_len$}", max_val).trim_end_matches(".0").to_string();
		let min_str = format!("{:.f_len$}", min_val).trim_end_matches(".0").to_string();
		let side_panel_width = max_str.len().max(min_str.len());
		let mut side_panel = String::with_capacity(height * side_panel_width);
		for i in 0..height {
			if i == 0 {
				side_panel.push_str(&format!("{max_str}\n"));
			} else if i == height - 1 {
				side_panel.push_str(&format!("{min_str}\n"));
			} else {
				side_panel.push_str(&format!("{:>side_panel_width$}\n", " "));
			}
		}
		side_panel.pop(); // remove last newline

		let mut plot_data = PlotData::new(min_val, max_val, height, use_font);

		// Check if we need to raise the plot
		let first_level = plot_data.get_level(prices[0], height - 1);
		let last_level = plot_data.get_level(prices[prices.len() - 1], height - 1);
		if first_level == 0 || last_level == 0 {
			plot_data.raise_plot();
		}

		let mut plot = Vec::with_capacity(height);

		for row in (0..height).rev() {
			let row_str: String = (0..width)
				.map(|j| {
					let index = (j as f64 * prices.len() as f64 / width as f64) as usize;
					let val = prices[index];
					let level = plot_data.get_level(val, row);
					plot_data.level_to_char(level)
				})
				.collect();
			plot.push(row_str);
		}

		join_str_blocks_v(plot.join("\n"), side_panel)
	}
}

fn join_str_blocks_v(left: String, right: String) -> String {
	assert_eq!(left.split('\n').count(), right.split('\n').count());
	left.lines().zip(right.lines()).map(|(l, r)| format!("{}{}", l, r)).collect::<Vec<String>>().join("\n")
}

/// # Panics
/// if ordinals on orders are outside of prices or not ascending.
///
/// # Blocker
/// Until better fonts, distinctions between price formats, multiple order lines at a time & order types, actual timeframes, are all extremely problematic; so their implementation is postponed.
///
/// # Architecture
/// Uses [SnapshotP] to build the plot, for finer control use it instead.
pub fn snapshot_plot_orders<T: Into<f64> + Copy>(prices: &[T], orders: &[(usize, Option<T>)]) -> String {
	let prices = prices.iter().map(|x| (*x).into()).collect::<Vec<f64>>();
	let orders = orders.iter().map(|(i, x)| (*i, x.map(|x| x.into()))).collect::<Vec<(usize, Option<f64>)>>();
	assert!(orders.iter().all(|(i, _)| *i < prices.len()));
	assert!(orders.windows(2).all(|w| w[0].0 < w[1].0));

	let mut order_points = Vec::with_capacity(prices.len());
	let mut last_order: (usize, Option<f64>) = (0, None);
	for (i, order) in orders.iter() {
		order_points.extend((last_order.0..*i).map(|_| last_order.1));
		last_order = (*i, *order);
	}
	order_points.extend((last_order.0..prices.len()).map(|_| last_order.1));

	SnapshotP::build(&prices).secondary_pane_optional(order_points).draw()
}

#[cfg(test)]
mod tests {
	use insta::assert_snapshot;
	use rand::{Rng, SeedableRng, rngs::StdRng};
	use v_utils::distributions::laplace_random_walk;

	use super::*;

	#[test]
	fn test_encode_decode_roundtrip() {
		for left in [0, 1, 125, 249, 250] {
			for right in [0, 1, 125, 249, 250] {
				let c = encode_bars(left, right);
				let (l, r) = decode_bars(c);
				assert_eq!((left, right), (l, r), "roundtrip failed for ({left}, {right})");
			}
		}
	}

	#[test]
	fn test_encode_pua_range() {
		// All codepoints should be in PUA-A range (U+F0000 - U+FFFFD)
		let min = encode_bars(0, 0);
		let max = encode_bars(250, 250);
		assert_eq!(min as u32, PUA_START, "min should be PUA_START");
		assert_eq!(max as u32, PUA_START + 250 * 251 + 250, "max should be PUA_START + 63000");
		assert!(max as u32 <= 0xffffd, "should not exceed PUA-A range");
	}

	#[test]
	fn test_snapshot_plot_p() {
		let data = laplace_random_walk(100.0, 1000, 0.1, 0.0, Some(42));
		let plot = SnapshotP::build(&data).draw();

		assert_snapshot!(plot, @r"
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󲩔󵢘󶅴󳸄󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀103.50
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󵖤󰀀󰀀󿘘󿘘󿘘󿘘󾩈󻈬󰿀󺥐󻰄󷨐󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󸛜󿘘󹞘󺕠󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󵒨󰀀󰀀󰀀󰀀󰀀󰀀󰀀󴷄󺥐󶝜󷰈      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󷀸󶅴󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󹞘󹚜󺝘󳘤󰀀󰀀󲍰󿘘󿘘󿘘󿘘      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󹒤󸾸󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󸇰󻻸󿘘󿘘󿘘󿘘󿘘      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󾕜󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󲍰󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󳴈󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󸏨󷌬󱞠󷬌󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󱂼󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󹲄󾁰󿘘󵖤󱢜󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󲁼󱖨󰀀󰀀󹂴󿘘󼓠󷨐󰀀󰀀󰀀󰟠󰀀󽮄󿘘󿘘󿘘󿘘󱦘󰀀󹦐󰀀󰀀󰀀󰀀󴟜󹒤󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘      
		󳄸󵊰󰀀󰀀󴗤󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󷌬󿘘󿘘󿘘󿘘󿘘󽆬󰀀󰀀󰀀󰀀󰀀󺸼󻌨󻻸󿘘󿘘󻿴󻄰󿘘󿘘󿘘󿘘󺱄󹪌󺍨󿘘󽞔󿘘󿘘󿘘󿘘󿘘󿘘󽆬󿘘󼛘󰀀󲹄󰏰󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘      
		󿘘󿘘󴧔󸧐󿘘󻴀󻄰󰀀󵊰󰀀󰀀󰀀󰀀󰀀󰀀󰀀󿘘󿘘󿘘󿘘󿘘󿘘󿘘󼶼󰀀󽒠󿌤󹎨󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󺵀󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘      
		󿘘󿘘󿘘󿘘󿘘󿘘󿘘󾴼󿘘󻻸󵚠󰀀󰟠󰀀󰀀󻷼󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󷠘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘      
		󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󼧌󿘘󻨌󽂰󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘98.73
		");
	}

	#[test]
	fn test_snapshot_plot_orders() {
		let prices = laplace_random_walk(100.0, 1000, 0.1, 0.0, Some(42));
		let n_orders = 10;
		let mut orders_left_to_select = 10;
		let mut order_ordinals = Vec::with_capacity(n_orders);
		for i in 0..prices.len() {
			let target_probability = orders_left_to_select as f64 / (prices.len() - i) as f64;
			let mut rng = StdRng::seed_from_u64(i as u64);
			if rng.random_range(0.0..1.0) < target_probability {
				order_ordinals.push(i);
				orders_left_to_select -= 1;
			}
		}
		let order_prices = laplace_random_walk(100.0, n_orders, 1.0, 0.0, Some(4));
		let mut orders = Vec::with_capacity(n_orders);
		for (i, o) in order_ordinals.iter().enumerate() {
			let order = match i == 6 || i == 7 {
				true => None,
				_ => Some(order_prices[i]),
			};
			orders.push((*o, order));
		}
		let plot = snapshot_plot_orders(&prices, &orders);
		insta::assert_snapshot!(plot, @r"
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󲩔󵢘󶅴󳸄󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀103.50
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󵖤󰀀󰀀󿘘󿘘󿘘󿘘󾩈󻈬󰿀󺥐󻰄󷨐󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󸛜󿘘󹞘󺕠󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󵒨󰀀󰀀󰀀󰀀󰀀󰀀󰀀󴷄󺥐󶝜󷰈      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󷀸󶅴󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󹞘󹚜󺝘󳘤󰀀󰀀󲍰󿘘󿘘󿘘󿘘      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󹒤󸾸󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󸇰󻻸󿘘󿘘󿘘󿘘󿘘      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󾕜󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󲍰󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󳴈󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󸏨󷌬󱞠󷬌󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󱂼󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󹲄󾁰󿘘󵖤󱢜󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󲁼󱖨󰀀󰀀󹂴󿘘󼓠󷨐󰀀󰀀󰀀󰟠󰀀󽮄󿘘󿘘󿘘󿘘󱦘󰀀󹦐󰀀󰀀󰀀󰀀󴟜󹒤󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘      
		󳄸󵊰󰀀󰀀󴗤󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󷌬󿘘󿘘󿘘󿘘󿘘󽆬󰀀󰀀󰀀󰀀󰀀󺸼󻌨󻻸󿘘󿘘󻿴󻄰󿘘󿘘󿘘󿘘󺱄󹪌󺍨󿘘󽞔󿘘󿘘󿘘󿘘󿘘󿘘󽆬󿘘󼛘󰀀󲹄󰏰󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘      
		󿘘󿘘󴧔󸧐󿘘󻴀󻄰󰀀󵊰󰀀󰀀󰀀󰀀󰀀󰀀󰀀󿘘󿘘󿘘󿘘󿘘󿘘󿘘󼶼󰀀󽒠󿌤󹎨󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󺵀󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘      
		󿘘󿘘󿘘󿘘󿘘󿘘󿘘󾴼󿘘󻻸󵚠󰀀󰟠󰀀󰀀󻷼󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󷠘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘      
		󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󼧌󿘘󻨌󽂰󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘98.73
		──────────────────────────────────────────────────────────────────────────────────────────
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󸾸󸾸󸾸󸾸󸾸󸾸󸾸󸾸󸾸󸾸󸾸󸾸󸾸󸾸󸾸󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀101.80
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󶉰󶉰󶉰󶉰󶉰󶉰󶉰󶉰󶉰󶉰󶉰󶉰󶉰󶉰󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󼳀󼗜󼗜󻿴󻿴󻿴󻿴󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀      
		󰀀󰀀󰀀󰀀󰀀󰀀󰀀󰀀󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󿘘󰀀󰀀󰀀󳌰󳌰󳌰󳌰󳌰󳌰󳌰󳌰󳌰󰃼󰃼󰃼󰃼󰃼󰃼󰃼󰃼󰃼󰃼󰃼󰃼󰃼󰃼󰃼󰃼󰃼󰃼󰃼󰃼97.82
		");
	}
}
