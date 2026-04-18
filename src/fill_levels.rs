use std::path::Path;

pub const PUA_START: u32 = 0xf09e5;
pub(crate) const LEVELS: u16 = 251;

const FALLBACK_BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
/// Generate the FillLevels TTF font file
pub fn generate_font(output: &Path) -> std::io::Result<()> {
	let glyph_script = format!(
		r#"
LEVELS = {levels}
PUA_START = {pua_start}
HALF_WIDTH = GLYPH_WIDTH // 2

for left in range(LEVELS):
    for right in range(LEVELS):
        char_code = PUA_START + left * LEVELS + right
        glyph = font.createChar(char_code)
        glyph.width = GLYPH_WIDTH

        # Draw from -HHEA_DESCENT to scaled height
        # Level 0 = empty, Level 250 = full bar from -483 to 1901
        left_height = int((left / 250.0) * LINE_HEIGHT) - HHEA_DESCENT
        right_height = int((right / 250.0) * LINE_HEIGHT) - HHEA_DESCENT

        pen = glyph.glyphPen()
        if left > 0:
            pen.moveTo((0, -HHEA_DESCENT))
            pen.lineTo((HALF_WIDTH, -HHEA_DESCENT))
            pen.lineTo((HALF_WIDTH, left_height))
            pen.lineTo((0, left_height))
            pen.closePath()

        if right > 0:
            pen.moveTo((HALF_WIDTH, -HHEA_DESCENT))
            pen.lineTo((GLYPH_WIDTH, -HHEA_DESCENT))
            pen.lineTo((GLYPH_WIDTH, right_height))
            pen.lineTo((HALF_WIDTH, right_height))
            pen.closePath()
        pen = None
"#,
		levels = LEVELS,
		pua_start = PUA_START,
	);

	crate::fontforge::generate_font("FillLevels", output, &glyph_script)
}
#[derive(bon::Builder, Clone, Debug, Default)]
pub struct SnapshotP {
	prices: Vec<f64>,
	secondary_pane: Option<Vec<Option<f64>>>,
	#[builder(default = SINGLE_PLOT_WIDTH)]
	width: usize,
	#[builder(default = SINGLE_PLOT_HEIGHT)]
	height: usize,
	#[builder(default)]
	fallback: bool,
}
impl SnapshotP {
	/// # Panics
	/// Meant to be used only in tests, so if any input params are incorrect we panic.
	pub fn draw(self) -> String {
		let use_font = !self.fallback;
		let main_section = Self::plot_p(self.prices, self.width, self.height, use_font);
		let mut out = main_section;
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
		let max_str = format!("{max_val:.f_len$}").trim_end_matches(".0").to_string();
		let min_str = format!("{min_val:.f_len$}").trim_end_matches(".0").to_string();
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
		let samples_per_char = plot_data.samples_per_char();
		let total_samples = width * samples_per_char;

		for row in (0..height).rev() {
			let row_str: String = (0..width)
				.map(|j| {
					let left_sample = j * samples_per_char;
					let right_sample = left_sample + samples_per_char - 1;
					let left_index = (left_sample as f64 * prices.len() as f64 / total_samples as f64) as usize;
					let right_index = (right_sample as f64 * prices.len() as f64 / total_samples as f64) as usize;
					let right_index = right_index.min(prices.len() - 1);

					match (prices[left_index], prices[right_index]) {
						(Some(left_val), Some(right_val)) => {
							let left_level = plot_data.get_level(left_val, row);
							let right_level = plot_data.get_level(right_val, row);
							plot_data.levels_to_char(left_level, right_level)
						}
						(Some(val), None) => {
							let level = plot_data.get_level(val, row);
							plot_data.levels_to_char(level, 0)
						}
						(None, Some(val)) => {
							let level = plot_data.get_level(val, row);
							plot_data.levels_to_char(0, level)
						}
						(None, None) => plot_data.empty_char(),
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
		let max_str = format!("{max_val:.f_len$}").trim_end_matches(".0").to_string();
		let min_str = format!("{min_val:.f_len$}").trim_end_matches(".0").to_string();
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
		let samples_per_char = plot_data.samples_per_char();
		let total_samples = width * samples_per_char;

		for row in (0..height).rev() {
			let row_str: String = (0..width)
				.map(|j| {
					let left_sample = j * samples_per_char;
					let right_sample = left_sample + samples_per_char - 1;
					let left_index = (left_sample as f64 * prices.len() as f64 / total_samples as f64) as usize;
					let right_index = (right_sample as f64 * prices.len() as f64 / total_samples as f64) as usize;
					let left_level = plot_data.get_level(prices[left_index], row);
					let right_level = plot_data.get_level(prices[right_index.min(prices.len() - 1)], row);
					plot_data.levels_to_char(left_level, right_level)
				})
				.collect();
			plot.push(row_str);
		}

		join_str_blocks_v(plot.join("\n"), side_panel)
	}
}

/// # Panics
/// if ordinals on orders are outside of prices or not ascending.
///
/// # Blocker
/// Until better fonts, distinctions between price formats, multiple order lines at a time & order types, actual timeframes, are all extremely problematic; so their implementation is postponed.
///
/// # Architecture
/// Uses [SnapshotP] to build the plot, for finer control use it instead.
pub fn snapshot_plot_orders<T: Into<f64> + Copy>(prices: &[T], orders: &[(usize, Option<T>)], fallback: bool) -> String {
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

	SnapshotP::builder().prices(prices).secondary_pane(order_points).fallback(fallback).build().draw()
}
#[cfg(test)]
fn decode_bars(c: char) -> (u8, u8) {
	let code = c as u32 - PUA_START;
	let left = (code / LEVELS as u32) as u8;
	let right = (code % LEVELS as u32) as u8;
	(left, right)
}

/// Encode two bar values (0-250 each) into a Unicode codepoint in PUA-A (Plane 15)
pub(crate) fn encode_bars(left: u8, right: u8) -> char {
	debug_assert!(left <= 250, "left must be 0-250");
	debug_assert!(right <= 250, "right must be 0-250");

	let code = PUA_START + left as u32 * LEVELS as u32 + right as u32;
	char::from_u32(code).expect("valid codepoint")
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
		let max_level = levels_per_row - 1;
		let data_range = max_val - min_val;
		// Use (height * max_level) so max value maps exactly to full bar
		let plot_range = (height * max_level) as f64;
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
		let max_level = self.levels_per_row - 1;
		(scaled_val - row as f64 * max_level as f64).clamp(0.0, max_level as f64) as u8
	}

	fn levels_to_char(&self, left: u8, right: u8) -> char {
		if self.use_font {
			encode_bars(left, right)
		} else {
			FALLBACK_BLOCKS[left.max(right) as usize]
		}
	}

	fn empty_char(&self) -> char {
		if self.use_font { encode_bars(0, 0) } else { ' ' }
	}

	/// Horizontal samples per character (2 for font mode, 1 for fallback)
	fn samples_per_char(&self) -> usize {
		if self.use_font { 2 } else { 1 }
	}

	/// Raise by the smallest step
	fn raise_plot(&mut self) {
		self.offset -= 1.0;
	}
}

fn join_str_blocks_v(left: String, right: String) -> String {
	assert_eq!(left.split('\n').count(), right.split('\n').count());
	left.lines().zip(right.lines()).map(|(l, r)| format!("{l}{r}")).collect::<Vec<String>>().join("\n")
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
		let plot_with_fallback = SnapshotP::builder().prices(data.clone()).fallback(true).build().draw();

		assert_snapshot!(plot_with_fallback, @"
		                                                                    ▂▃▄▃                  103.50
		                                                                 ▃  █████▆▁▆▇▄                  
		                                                                ▅█▅▆██████████▃       ▃▆▄▄      
		                                                              ▄▄███████████████▅▅▆▂  ▂████      
		                                                            ▅▅█████████████████████▅▇█████      
		                                                           ███████████████████████████████      
		                   ▂                ▂        ▅▄▁▄         ▁███████████████████████████████      
		                 ▆██▃▁         ▂▁  ▅█▇▄   ▁ █████▁ ▅    ▃▅████████████████████████████████      
		▂▃  ▃           ▄█████▇     ▆▆▇██▇▆████▆▅▆█▇██████▇█▇ ▂▁██████████████████████████████████      
		██▃▅█▇▆ ▃       ███████▇ ▇█▅█████████████████████████▆████████████████████████████████████      
		█████████▇▃ ▁  ▇████████▄█████████████████████████████████████████████████████████████████      
		███████████▇█▇▇███████████████████████████████████████████████████████████████████████████98.73
		");

		let plot_no_fallback = SnapshotP::builder().prices(data).fallback(false).build().draw();
		assert_snapshot!(plot_no_fallback, @r"
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󳎶󶅇󶭞󴞪󰧥󰧲󰧥󰨫󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥103.50
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󵾊󰧥󰩌󿿽󿿽󿿽󿿘󿌯󻭛󱨃󻊈󼔁󸎂󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󹁽󿿽󺂇󺺜󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󵸷󰧥󰧥󰧥󰧥󰧥󰧥󰧥󵞂󻇶󷆰󸒛
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧴󷦎󶩡󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿾴󺄞󹽴󻃋󴀁󰧥󰧥󲸠󿿽󿿽󿿽󿿽
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰪮󹶼󹨅󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󸪉󼟬󿿽󿿽󿿽󿿽󿿽
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧵󾹗󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰩰󲴱󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰨅󴜪󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰨏󸱧󷰁󲆇󸍹󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󱮊󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󺖢󾩫󿿺󵼲󲉳󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧿󲦑󱽷󰧥󰧥󹨅󿿽󼷋󸌤󰧥󰧥󰨦󱈦󰧻󾖄󿿽󿿽󿿽󿾓󲎄󰧭󺊼󰧥󰧥󰧥󰧥󵇖󹺟󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽
		󳫫󵱁󰧥󰧩󴺇󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󷲒󿿽󿿽󿿽󿾷󿿽󽧁󰧥󰧥󰧥󰩶󰨪󻛄󻯏󼠚󿿽󿿹󼣂󻩌󿿽󿿽󿿽󿿽󻘩󺎈󺲦󿿽󾂝󿿽󿿽󿿽󿿽󿿽󿿽󽪻󿿽󼼓󰧥󳠠󰹸󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽
		󿾳󿾽󵐆󹌨󿿥󼕪󻥻󰧥󵭆󰧥󰧥󰧥󰧥󰧥󰧥󰩱󿿽󿿽󿿽󿿽󿿽󿿽󿼳󽚂󰧥󽵂󿰑󹳶󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󻜋󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽
		󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿘟󿿄󼝚󵾛󰧥󱆽󰧥󰩬󼜟󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󸈙󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽
		󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󽋃󿿤󼉝󽧀󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽98.73
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
		let plot = snapshot_plot_orders(&prices, &orders, false);
		insta::assert_snapshot!(plot, @"
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󳎶󶅇󶭞󴞪󰧥󰧲󰧥󰨫󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥103.50
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󵾊󰧥󰩌󿿽󿿽󿿽󿿘󿌯󻭛󱨃󻊈󼔁󸎂󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥      
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󹁽󿿽󺂇󺺜󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󵸷󰧥󰧥󰧥󰧥󰧥󰧥󰧥󵞂󻇶󷆰󸒛      
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧴󷦎󶩡󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿾴󺄞󹽴󻃋󴀁󰧥󰧥󲸠󿿽󿿽󿿽󿿽      
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰪮󹶼󹨅󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󸪉󼟬󿿽󿿽󿿽󿿽󿿽      
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧵󾹗󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽      
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰩰󲴱󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰨅󴜪󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰨏󸱧󷰁󲆇󸍹󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󱮊󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽      
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󺖢󾩫󿿺󵼲󲉳󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧿󲦑󱽷󰧥󰧥󹨅󿿽󼷋󸌤󰧥󰧥󰨦󱈦󰧻󾖄󿿽󿿽󿿽󿾓󲎄󰧭󺊼󰧥󰧥󰧥󰧥󵇖󹺟󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽      
		󳫫󵱁󰧥󰧩󴺇󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󷲒󿿽󿿽󿿽󿾷󿿽󽧁󰧥󰧥󰧥󰩶󰨪󻛄󻯏󼠚󿿽󿿹󼣂󻩌󿿽󿿽󿿽󿿽󻘩󺎈󺲦󿿽󾂝󿿽󿿽󿿽󿿽󿿽󿿽󽪻󿿽󼼓󰧥󳠠󰹸󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽      
		󿾳󿾽󵐆󹌨󿿥󼕪󻥻󰧥󵭆󰧥󰧥󰧥󰧥󰧥󰧥󰩱󿿽󿿽󿿽󿿽󿿽󿿽󿼳󽚂󰧥󽵂󿰑󹳶󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󻜋󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽      
		󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿘟󿿄󼝚󵾛󰧥󱆽󰧥󰩬󼜟󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󸈙󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽      
		󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󽋃󿿤󼉝󽧀󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽98.73
		──────────────────────────────────────────────────────────────────────────────────────────
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰩶󹢡󹢡󹢡󹢡󹢡󹢡󹢡󹢡󹢡󹢡󹢡󹢡󹢡󹢡󹢡󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿼃󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥101.80
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰩈󶭙󶭙󶭙󶭙󶭙󶭙󶭙󶭙󶭙󶭙󶭙󶭙󶭙󶯰󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿼃󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥      
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰫟󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿼃󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥      
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󽚞󼿁󼾺󼣝󼣝󼣝󼤕󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿼃󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥      
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿼃󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥      
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿼃󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥      
		󰧥󰧥󰧥󰧥󰧥󰧥󰧥󰧥󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿿽󿼃󰧥󰧥󰨘󳰙󳰙󳰙󳰙󳰙󳰙󳰙󳰙󳰙󰫡󰫡󰫡󰫡󰫡󰫡󰫡󰫡󰫡󰫡󰫡󰫡󰫡󰫡󰫡󰫡󰫡󰫡󰫡󰫡97.82
		");
	}
}
