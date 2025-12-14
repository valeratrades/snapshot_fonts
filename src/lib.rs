use std::{
	io::Write,
	path::Path,
	process::{Command, Stdio},
};

pub const LEVELS: u16 = 251;
pub const SURROGATE_START: u32 = 0xd800;
pub const SURROGATE_LEN: u32 = 2048;

/// Encode two bar values (0-250 each) into a Unicode codepoint
pub fn encode_bars(left: u8, right: u8) -> char {
	debug_assert!(left <= 250, "left must be 0-250");
	debug_assert!(right <= 250, "right must be 0-250");

	let mut code = left as u32 * LEVELS as u32 + right as u32;
	if code >= SURROGATE_START {
		code += SURROGATE_LEN;
	}
	char::from_u32(code).expect("valid codepoint")
}

/// Decode a Unicode codepoint back to two bar values
pub fn decode_bars(c: char) -> (u8, u8) {
	let mut code = c as u32;
	if code >= SURROGATE_START + SURROGATE_LEN {
		code -= SURROGATE_LEN;
	}
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

font.generate("{output}")
"#,
		levels = LEVELS,
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
	if status.success() {
		Ok(())
	} else {
		Err(std::io::Error::new(std::io::ErrorKind::Other, "fontforge failed"))
	}
}

// ============================================================================
// Snapshot plotting (copied from v_utils/snapshots.rs)
// ============================================================================

struct PlotData {
	scale: f64,
	offset: f64,
	blocks: [char; 9],
}

static SINGLE_PLOT_WIDTH: usize = 90;
static SINGLE_PLOT_HEIGHT: usize = 12;

impl PlotData {
	fn new(min_val: f64, max_val: f64, height: usize) -> Self {
		let data_range = max_val - min_val;
		let plot_range = (height * 8) as f64;
		let scale = plot_range / data_range;
		let offset = min_val * scale;
		let blocks = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
		PlotData { scale, offset, blocks }
	}

	fn get_block_index(&self, val: f64, i: usize) -> usize {
		let scaled_val = val * self.scale - self.offset;
		(scaled_val - i as f64 * 8.0).clamp(0.0, 8.0) as usize
	}

	/// Raise by the smallest step (▁)
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
		let main_section = Self::plot_p(self.prices, self.width, self.height);
		let mut out = main_section;
		if let Some(secondary_pane) = self.secondary_pane {
			let separator = "─".repeat(self.width);
			let secondary_section = Self::plot_p_optional(secondary_pane, self.width, (self.height * 3) / 5);
			out.push_str(&format!("\n{separator}\n{secondary_section}"));
		}
		out
	}

	fn plot_p_optional(prices: Vec<Option<f64>>, width: usize, height: usize) -> String {
		if prices.is_empty() {
			return " ".repeat(width).repeat(height);
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
			return " ".repeat(width).repeat(height);
		}

		let mut plot_data = PlotData::new(min_val, max_val, height);
		plot_data.raise_plot(); // here we always want to raise to be able to distinguish between empty and non-empty prices

		let mut plot = Vec::with_capacity(height);
		for i in (0..height).rev() {
			let row: String = (0..width)
				.map(|j| {
					let index = (j as f64 * prices.len() as f64 / width as f64) as usize;
					match prices[index] {
						Some(val) => {
							let block_index = plot_data.get_block_index(val, i);
							plot_data.blocks[block_index]
						}
						None => ' ',
					}
				})
				.collect();
			plot.push(row);
		}

		join_str_blocks_v(plot.join("\n"), side_panel)
	}

	fn plot_p(prices: Vec<f64>, width: usize, height: usize) -> String {
		if prices.is_empty() {
			panic!("prices are empty");
		}

		let min_val = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
		let max_val = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
		if (max_val - min_val).abs() < f64::EPSILON {
			return " ".repeat(width).repeat(height);
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

		let mut plot_data = PlotData::new(min_val, max_val, height);

		// Check if we need to raise the plot
		let first_block = plot_data.get_block_index(prices[0], height - 1);
		let last_block = plot_data.get_block_index(prices[prices.len() - 1], height - 1);
		if first_block == 0 || last_block == 0 {
			plot_data.raise_plot();
		}

		let mut plot = Vec::with_capacity(height);

		for i in (0..height).rev() {
			let row: String = (0..width)
				.map(|j| {
					let index = (j as f64 * prices.len() as f64 / width as f64) as usize;
					let val = prices[index];
					let block_index = plot_data.get_block_index(val, i);
					plot_data.blocks[block_index]
				})
				.collect();
			plot.push(row);
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
				assert_eq!((left, right), (l, r), "roundtrip failed for ({}, {})", left, right);
			}
		}
	}

	#[test]
	fn test_encode_surrogate_skip() {
		// Code 55296 (0xD800) should be skipped
		// left=220, right=1: 220*251 + 1 = 55221 (before surrogate)
		// left=220, right=76: 220*251 + 76 = 55296 (would be surrogate, should skip)
		let c = encode_bars(220, 76);
		assert!(c as u32 >= SURROGATE_START + SURROGATE_LEN, "should skip surrogate range");
	}

	#[test]
	fn test_snapshot_plot_p() {
		let data = laplace_random_walk(100.0, 1000, 0.1, 0.0, Some(42));
		let plot = SnapshotP::build(&data).draw();

		assert_snapshot!(plot, @r"
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
		──────────────────────────────────────────────────────────────────────────────────────────
		                             ▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅██████████████                                101.80
		               ▄▄▄▄▄▄▄▄▄▄▄▄▄▄█████████████████████████████                                      
		               ███████████████████████████████████████████                                      
		        ▇▇▇▇▇▇▇███████████████████████████████████████████                                      
		        ██████████████████████████████████████████████████                                      
		        ██████████████████████████████████████████████████                                      
		        ██████████████████████████████████████████████████   ▂▂▂▂▂▂▂▂▂▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁97.82
		");
	}
}
