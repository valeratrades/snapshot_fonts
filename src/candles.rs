use std::path::Path;

use crate::fontforge::{GLYPH_WIDTH, HHEA_DESCENT, LINE_HEIGHT};

/// Candle size (max height = 11, giving 12 placement positions 0..=11)
pub const CANDLE_SIZE: u8 = 11;
/// Total number of candle glyphs (calculated by calc_candles script: 52416 + 1 empty = 52417)
pub const CANDLE_GLYPH_COUNT: u32 = 52417;
/// Start of candle font in PUA-B (Plane 16): U+100000–U+10FFFD
pub const CANDLE_PUA_START: u32 = 0x100000;

// Derived constants for glyph geometry
const WICK_LEFT: i32 = GLYPH_WIDTH / 3;
const WICK_RIGHT: i32 = 2 * GLYPH_WIDTH / 3;
const BODY_LEFT: i32 = 0;
const BODY_RIGHT: i32 = GLYPH_WIDTH;
const LEVEL_HEIGHT: i32 = LINE_HEIGHT / CANDLE_SIZE as i32;
const DOJI_HEIGHT: i32 = LINE_HEIGHT / 44;

/// A rectangle defined by its corners
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rect {
	left: i32,
	bottom: i32,
	right: i32,
	top: i32,
}

impl Rect {
	fn is_valid(&self) -> bool {
		self.top > self.bottom && self.right > self.left
	}
}

/// Precomputed glyph geometry for a single candle
#[derive(Clone, Copy, Debug)]
struct CandleGlyph {
	char_code: u32,
	wick: Option<Rect>,
	body: Option<Rect>,
}

impl CandleGlyph {
	fn as_python(&self) -> String {
		let mut lines = Vec::new();
		lines.push(format!("g=font.createChar({});g.width={}", self.char_code, GLYPH_WIDTH));

		if self.wick.is_some() || self.body.is_some() {
			lines.push("p=g.glyphPen()".to_string());

			if let Some(w) = self.wick {
				lines.push(format!(
					"p.moveTo(({},{}));p.lineTo(({},{}));p.lineTo(({},{}));p.lineTo(({},{}));p.closePath()",
					w.left, w.bottom, w.right, w.bottom, w.right, w.top, w.left, w.top
				));
			}

			if let Some(b) = self.body {
				lines.push(format!(
					"p.moveTo(({},{}));p.lineTo(({},{}));p.lineTo(({},{}));p.lineTo(({},{}));p.closePath()",
					b.left, b.bottom, b.right, b.bottom, b.right, b.top, b.left, b.top
				));
			}

			lines.push("p=None".to_string());
		}

		lines.join("\n")
	}
}

/// Generate all candle glyphs with precomputed geometry
fn generate_all_glyphs() -> Vec<CandleGlyph> {
	let mut glyphs = Vec::with_capacity(CANDLE_GLYPH_COUNT as usize);
	let mut char_code = CANDLE_PUA_START;
	let size = CANDLE_SIZE as i32;

	// Index 0: empty candle
	glyphs.push(CandleGlyph { char_code, wick: None, body: None });
	char_code += 1;

	// For each height h (0..=SIZE)
	for h in 0..=size {
		let n_borders = h + 1;

		// For each placement p (0..=SIZE)
		for p in 0..=size {
			let base_y = -HHEA_DESCENT + p * LEVEL_HEIGHT;

			// For each body configuration
			for body_start in 0..n_borders {
				for body_size in 0..(n_borders - body_start) {
					let options_wick_above = 1 + body_start;
					let options_wick_below = n_borders - (body_start + body_size);

					// Generate all wick combinations
					for wick_above in 0..options_wick_above {
						for wick_below in 0..options_wick_below {
							let candle_top = base_y + h * LEVEL_HEIGHT;
							let candle_bottom = base_y;

							// Body position
							let body_top_y = candle_top - body_start * LEVEL_HEIGHT;
							let body_bottom_y = if body_size == 0 { body_top_y - DOJI_HEIGHT } else { body_top_y - body_size * LEVEL_HEIGHT };

							// Wick position (clamped to candle bounds)
							let wick_top_y = (body_top_y + wick_above * LEVEL_HEIGHT).min(candle_top);
							let wick_bottom_y = (body_bottom_y - wick_below * LEVEL_HEIGHT).max(candle_bottom);

							let wick = Rect {
								left: WICK_LEFT,
								right: WICK_RIGHT,
								bottom: wick_bottom_y,
								top: wick_top_y,
							};
							let body = Rect {
								left: BODY_LEFT,
								right: BODY_RIGHT,
								bottom: body_bottom_y,
								top: body_top_y,
							};

							glyphs.push(CandleGlyph {
								char_code,
								wick: wick.is_valid().then_some(wick),
								body: body.is_valid().then_some(body),
							});
							char_code += 1;
						}
					}
				}
			}
		}
	}

	glyphs
}

/// Candle representation using the encoding from calc_candles:
/// - `placement`: 0-11, vertical position of candle within character cell
/// - `height`: 0-11, internal candle height (wick span from high to low)
/// - `body_start`: offset from top of candle to body top (0 to height)
/// - `body_size`: size of body (0 = doji, drawn as thin line)
/// - `wick_above`: how far wick extends above body (0 to body_start)
/// - `wick_below`: how far wick extends below body (0 to height - body_start - body_size)
///
/// For simplicity, we use the first wick configuration (wick_above=0, wick_below=0)
/// in the basic encode/decode. Full wick support would need additional parameters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Candle {
	pub placement: u8,
	pub height: u8,
	pub body_start: u8,
	pub body_size: u8,
}

impl Candle {
	pub fn new(placement: u8, height: u8, body_start: u8, body_size: u8) -> Self {
		debug_assert!(placement <= CANDLE_SIZE);
		debug_assert!(height <= CANDLE_SIZE);
		debug_assert!(body_start <= height);
		debug_assert!(body_start + body_size <= height);
		Candle {
			placement,
			height,
			body_start,
			body_size,
		}
	}

	/// Create empty candle (codepoint 0 in candle range)
	pub fn empty() -> Self {
		Candle {
			placement: 0,
			height: 0,
			body_start: 0,
			body_size: 0,
		}
	}

	/// Check if this is the empty candle
	pub fn is_empty(&self) -> bool {
		// Empty candle is index 0, which is height=0, placement=0, body=0
		self.height == 0 && self.placement == 0 && self.body_start == 0 && self.body_size == 0
	}
}

// ============================================================================
// Chart rendering
// ============================================================================

use v_utils::trades::Ohlc;

const DEFAULT_WIDTH: usize = 90;
const DEFAULT_HEIGHT: usize = 12;

/// Builder for candlestick chart snapshots
#[derive(Clone, Debug)]
pub struct SnapshotCandles {
	ohlcs: Vec<Ohlc>,
	width: usize,
	height: usize,
}

impl SnapshotCandles {
	/// Create from OHLC data
	pub fn from_ohlc(ohlcs: &[Ohlc]) -> Self {
		Self {
			ohlcs: ohlcs.to_vec(),
			width: DEFAULT_WIDTH,
			height: DEFAULT_HEIGHT,
		}
	}

	/// Create from price series - step size is calculated from width
	pub fn from_prices<T: Into<f64> + Copy>(prices: &[T]) -> Self {
		let prices: Vec<f64> = prices.iter().map(|p| (*p).into()).collect();
		let step = (prices.len() / DEFAULT_WIDTH).max(1);
		let ohlcs = v_utils::trades::mock_p_to_ohlc(&prices, step);
		Self {
			ohlcs,
			width: DEFAULT_WIDTH,
			height: DEFAULT_HEIGHT,
		}
	}

	pub fn width(mut self, width: usize) -> Self {
		self.width = width;
		self
	}

	pub fn height(mut self, height: usize) -> Self {
		self.height = height;
		self
	}

	/// Render the candlestick chart
	/// Multiple rows give more vertical precision - each row covers (CANDLE_SIZE+1) levels
	pub fn draw(&self) -> String {
		let empty = encode_candle(Candle::empty());
		if self.ohlcs.is_empty() {
			return (0..self.height).map(|_| empty.to_string().repeat(self.width)).collect::<Vec<_>>().join("\n");
		}

		// Find price range from wick extremes
		let min_price = self.ohlcs.iter().map(|o| o.low).fold(f64::INFINITY, f64::min);
		let max_price = self.ohlcs.iter().map(|o| o.high).fold(f64::NEG_INFINITY, f64::max);

		if (max_price - min_price).abs() < f64::EPSILON {
			let mid_candle = Candle::new(CANDLE_SIZE / 2, 0, 0, 0);
			let mid = encode_candle(mid_candle);
			return (0..self.height).map(|_| mid.to_string().repeat(self.width)).collect::<Vec<_>>().join("\n");
		}

		// Total levels = height rows * 11 levels per row
		let levels_per_row = CANDLE_SIZE as usize;
		let total_levels = self.height * levels_per_row;
		let price_per_level = (max_price - min_price) / (total_levels - 1) as f64;

		// For each column, compute the OHLC levels
		let mut col_data: Vec<(usize, usize, usize, usize)> = Vec::with_capacity(self.width);
		for i in 0..self.width {
			let ohlc_idx = (i * self.ohlcs.len()) / self.width;
			let ohlc = &self.ohlcs[ohlc_idx.min(self.ohlcs.len() - 1)];

			let high_level = ((ohlc.high - min_price) / price_per_level).round() as usize;
			let low_level = ((ohlc.low - min_price) / price_per_level).round() as usize;
			let open_level = ((ohlc.open - min_price) / price_per_level).round() as usize;
			let close_level = ((ohlc.close - min_price) / price_per_level).round() as usize;

			col_data.push((
				high_level.min(total_levels - 1),
				low_level.min(total_levels - 1),
				open_level.min(total_levels - 1),
				close_level.min(total_levels - 1),
			));
		}

		// Build rows from top to bottom
		let mut rows: Vec<String> = Vec::with_capacity(self.height);
		for row in (0..self.height).rev() {
			let row_bottom = row * levels_per_row;
			let row_top = row_bottom + levels_per_row - 1;

			let mut row_chars: Vec<char> = Vec::with_capacity(self.width);
			for &(high, low, open, close) in &col_data {
				// Does this candle's wick intersect this row?
				if high < row_bottom || low > row_top {
					row_chars.push(empty);
					continue;
				}

				// Determine if candle extends beyond this row
				let extends_below = low < row_bottom;
				let extends_above = high > row_top;

				// For visual continuity of multi-row candles, fill the entire cell
				// when candle extends to adjacent rows
				let fills_cell = extends_below || extends_above;

				// Wick bounds within this row
				let local_low = if fills_cell { 0 } else { low - row_bottom };
				let local_high = if fills_cell { CANDLE_SIZE as usize } else { high - row_bottom };

				// placement = where candle low sits in this row (0-11)
				let placement = local_low;

				// height = wick span within this row
				let height = local_high.saturating_sub(local_low);

				// Body bounds (in global levels)
				let body_top_global = open.max(close);
				let body_bottom_global = open.min(close);

				// Check if body intersects this row
				let (body_start, body_size) = if body_top_global < row_bottom || body_bottom_global > row_top {
					// Body doesn't intersect this row - just wick
					(0, 0)
				} else {
					// Body intersects - clamp to row
					let local_body_bottom = if body_bottom_global < row_bottom { 0 } else { body_bottom_global - row_bottom };
					let local_body_top = if body_top_global > row_top { CANDLE_SIZE as usize } else { body_top_global - row_bottom };

					// body_start = offset from wick top to body top
					let bs = local_high.saturating_sub(local_body_top).min(height);
					// body_size = body span
					let bsz = local_body_top.saturating_sub(local_body_bottom).min(height.saturating_sub(bs));
					(bs, bsz)
				};

				let candle = Candle::new(placement as u8, height as u8, body_start as u8, body_size as u8);
				row_chars.push(encode_candle(candle));
			}
			rows.push(row_chars.iter().collect());
		}

		rows.join("\n")
	}
}

/// Encode a candle into a Unicode codepoint in PUA
/// Index 0 = empty candle
/// Then for each height h (0..=11):
///   for each placement p (0..=11):
///     for each body config with wick options
pub fn encode_candle(candle: Candle) -> char {
	// Index 0 is empty
	if candle.is_empty() {
		return char::from_u32(CANDLE_PUA_START).expect("valid codepoint");
	}

	let mut index: u32 = 1; // Start at 1 (0 is empty)
	let size = CANDLE_SIZE as u32;
	let placement_options = size + 1; // 12

	// Add all glyphs from previous heights
	for h in 0..candle.height {
		index += candle_count_for_height(h as u32, size) * placement_options;
	}

	// Add glyphs from previous placements within this height
	let h = candle.height as u32;
	index += candle.placement as u32 * candle_count_for_height(h, size);

	// Add glyphs from previous body configs within this height/placement
	// Iterate body_start from 0
	let n_borders = h + 1;
	for bs in 0..candle.body_start as u32 {
		for bsz in 0..(n_borders - bs) {
			index += wick_options(bs, bsz, n_borders);
		}
	}

	// Add glyphs from previous body_size within this body_start
	for bsz in 0..candle.body_size as u32 {
		index += wick_options(candle.body_start as u32, bsz, n_borders);
	}

	// We use first wick option (wick_above=0, wick_below=0)
	let code = CANDLE_PUA_START + index;
	char::from_u32(code).expect("valid codepoint")
}

/// Decode a Unicode codepoint back to a Candle
pub fn decode_candle(c: char) -> Candle {
	let index = c as u32 - CANDLE_PUA_START;

	// Index 0 is empty
	if index == 0 {
		return Candle::empty();
	}

	let mut remaining = index - 1; // Subtract 1 for empty
	let size = CANDLE_SIZE as u32;
	let placement_options = size + 1;

	// Find height
	let mut height: u32 = 0;
	while height <= size {
		let count = candle_count_for_height(height, size) * placement_options;
		if remaining < count {
			break;
		}
		remaining -= count;
		height += 1;
	}

	if height > size {
		return Candle::empty();
	}

	// Find placement within this height
	let count_per_placement = candle_count_for_height(height, size);
	let placement = remaining / count_per_placement;
	remaining %= count_per_placement;

	// Find body_start and body_size
	let n_borders = height + 1;
	let mut body_start: u32 = 0;
	while body_start < n_borders {
		let mut count_for_bs: u32 = 0;
		for bsz in 0..(n_borders - body_start) {
			count_for_bs += wick_options(body_start, bsz, n_borders);
		}
		if remaining < count_for_bs {
			break;
		}
		remaining -= count_for_bs;
		body_start += 1;
	}

	let mut body_size: u32 = 0;
	while body_start + body_size < n_borders {
		let count = wick_options(body_start, body_size, n_borders);
		if remaining < count {
			break;
		}
		remaining -= count;
		body_size += 1;
	}

	Candle::new(placement as u8, height as u8, body_start as u8, body_size as u8)
}

/// Count candle body/wick configurations for a given height (not including placement)
fn candle_count_for_height(height: u32, _size: u32) -> u32 {
	let n_borders = height + 1;
	let mut total: u32 = 0;
	for open_offset_i in 0..n_borders {
		for close_from_open_j in 0..(n_borders - open_offset_i) {
			total += wick_options(open_offset_i, close_from_open_j, n_borders);
		}
	}
	total
}

/// Calculate wick placement options
/// open_offset_i = body_start (distance from top)
/// close_from_open_j = body_size
fn wick_options(open_offset_i: u32, close_from_open_j: u32, n_borders: u32) -> u32 {
	let options_wick_above = 1 + open_offset_i;
	let options_wick_below = n_borders - (open_offset_i + close_from_open_j);
	options_wick_above * options_wick_below
}

/// Generate the glyph script with all geometry precomputed in Rust
pub fn generate_glyph_script() -> String {
	let glyphs = generate_all_glyphs();
	glyphs.iter().map(|g| g.as_python()).collect::<Vec<_>>().join("\n")
}

/// Generate the Candles TTF font file
/// Each glyph represents a candlestick with:
/// - Horizontal space split in 3: left wick area, body (middle 1/3), right wick area
/// - Wick is drawn in the center third
/// - Body is drawn wider, covering the full width
/// - If body_size == 0 (doji), body is drawn as 1/44 of char height
///
/// Encoding order (matching Rust encode_candle):
/// - Index 0: empty candle
/// - For each height h (0..=SIZE):
///   - For each placement p (0..=SIZE):
///     - For each body_start (0..=h):
///       - For each body_size (0..=(h-body_start)):
///         - For each wick_above (0..body_start):
///           - For each wick_below (0..=(h-body_start-body_size)):
///             - One glyph
pub fn generate_candle_font(output: &Path) -> std::io::Result<()> {
	let glyph_script = generate_glyph_script();
	crate::fontforge::generate_font("Candles", output, &glyph_script)
}

#[cfg(test)]
mod tests {
	use insta::assert_snapshot;

	use super::*;

	fn test_chart() -> String {
		use v_utils::distributions::laplace_random_walk;
		let prices = laplace_random_walk(100.0, 1000, 0.1, 0.0, Some(42));
		SnapshotCandles::from_prices(&prices).draw()
	}

	#[test]
	fn test_snapshot_candles_chart() {
		let chart = test_chart();

		assert_snapshot!(chart, @r"
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􉇣􉆨􉅏􉆨􈳅􉇣􈳅􀀀􉈃􈳅􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􉈃􉇣􀀀􈴒􈳦􈳦􈳜􈴩􈴉􉅻􀲼􈳅􈾕􉈎􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􈳅􈴒􈳯􀎁􈳑􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􈴍􀀀􀀀􈳅􀀀􀀀􀀀􀀀􉆨􈽲􀞫􃸍
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􉈎􂕍􀻢􈸪􈳯􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀽝􀜘􈾤􀍎􉆨􀀀􉈃􈴐􈳅􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􉅏􀝩􈳦􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􈳾􀶂􈳑􀀀􀀀􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􈴒􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􉈎􉈃􈳅􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􈳅􉅏􉆨􀀀􀀀􀀀􀀀􀀀􀀀􉈃􀚦􂛚􉅏􉈃􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􈻬􈳯􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􉈃􈴄􈳑􈴽􈾟􉈃􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􉇣􀘚􉄿􉈎􀀀􈻘􈳦􈳦􀹯􈾟􈳅􉇣􀘚􉈃􈴍􀀀􀀀􈳑􈳷􉁆􉈃􅸚􈸹􀀀􀀀􀀀􉅏􀎔􈳾􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􉇏􉆨􀀀􉈎􉆨􈳅􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􈳅􈴐􀀀􀀀􀀀􈳦􈳦􈻬􀀀􀀀􀀀􉆨􉀐􀶑􀇀􈳦􀀀􈳅􈳷􀼴􀀀􀀀􀀀􀀀􈳯􈼘􈳦􀀀􈳑􀀀􀀀􀀀􀀀􀀀􈳦􈳯􀀀􈴐􉇭􉈎􉁆􈳯􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􈳅􈳦􀞱􈳷􈳦􈶵􄀚􉈃􉆖􈳅􀀀􀀀􀀀􀀀􀀀􈶕􀀀􀀀􀀀􀀀􀀀􀀀􈴉􂠩􉈃􃹿􈴍􈳅􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􈳅􈳑􈳑􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􈳑􈳅􈵹􉃘􉈎􈳅􀀀􈻬􈳜􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􈳯􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􈳅􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􈳯􈳷􈴓􃴴􈴉􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		");
	}

	/// Single candle spanning 3 rows - must render as continuous vertical bar
	/// From debug: OHLC open=99.96, high=99.96, low=99.51, close=99.95
	/// With price range ~98.73 to ~103.50, this spans global levels 23-37
	/// which crosses rows 8, 9, 10 (each row = 12 levels)
	#[test]
	fn test_single_candle_spanning_three_rows() {
		let chart = test_chart();

		#[rustfmt::skip]
		let problematic_col: Vec<char> = chart
			.lines()
			.filter_map(|line| line.chars().nth(53))
			.collect();

		let decoded: Vec<String> = problematic_col
			.iter()
			.enumerate()
			.map(|(i, &c)| {
				let candle = decode_candle(c);
				if candle.is_empty() {
					format!("row{i}:empty")
				} else {
					format!("row{i}:p={},h={},bs={},bsz={}", candle.placement, candle.height, candle.body_start, candle.body_size)
				}
			})
			.collect();

		assert_snapshot!(decoded.join("\n"), @r"
		row0:empty
		row1:empty
		row2:empty
		row3:empty
		row4:empty
		row5:empty
		row6:empty
		row7:empty
		row8:p=0,h=11,bs=10,bsz=0
		row9:p=0,h=11,bs=0,bsz=0
		row10:p=0,h=11,bs=0,bsz=0
		row11:empty
		");
		assert_snapshot!(problematic_col.iter().collect::<String>(), @"􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􉇭􈳅􈳅􀀀");
		assert_eq!(problematic_col[9], encode_candle(Candle::new(0, 11, 0, 11))); //NB: message for Claude: do not change, this is correct. If you think it's wrong, - fucking stop what you are doing and await further instructions, - you are very very very confused and must not make any more changes.
	}

	/// Test that multi-row candles are properly connected:
	/// - Adjacent rows in the same column must be directly touching
	/// - Candle above must have its bottom extend fully down (placement = 0)
	/// - Candle below must have its top extend fully up (placement + height = CANDLE_SIZE)
	//TODO!!!!!!!!!!!!!!!: this test is WRONG. MUST be failing with current implementation. Fix the test to correctly detect candles not feeling cleanly.
	#[test]
	fn test_multirow_candle_continuity() {
		let chart = test_chart();
		let rows: Vec<&str> = chart.lines().collect();
		let height = rows.len();

		if height < 2 {
			return;
		}

		// Parse all rows into decoded candles, keeping original chars for debug
		let decoded: Vec<Vec<(char, Candle)>> = rows.iter().map(|row| row.chars().map(|c| (c, decode_candle(c))).collect()).collect();

		let width = decoded[0].len();
		let mut errors: Vec<String> = Vec::new();
		let mut adjacent_pairs = 0;

		// Check each column
		for col in 0..width {
			// Check each pair of vertically adjacent rows
			for row_above_idx in 0..(height - 1) {
				let row_below_idx = row_above_idx + 1;
				let (char_above, candle_above) = decoded[row_above_idx][col];
				let (char_below, candle_below) = decoded[row_below_idx][col];

				// Skip if either is empty
				if candle_above.is_empty() || candle_below.is_empty() {
					continue;
				}

				adjacent_pairs += 1;

				// Both cells have candles - they MUST connect at the row boundary
				// For connection: above candle must span full cell (touch bottom edge)
				//                 below candle must span full cell (touch top edge)

				let above_top = candle_above.placement + candle_above.height;
				let below_top = candle_below.placement + candle_below.height;

				// Candle above must fill its cell completely to connect downward
				if candle_above.placement != 0 || above_top != CANDLE_SIZE {
					errors.push(format!(
						"Col {col}, row {row_above_idx}: above '{char_above}' p={} h={} doesn't fill cell (need p=0, p+h={})",
						candle_above.placement, candle_above.height, CANDLE_SIZE
					));
				}

				// Candle below must fill its cell completely to connect upward
				if candle_below.placement != 0 || below_top != CANDLE_SIZE {
					errors.push(format!(
						"Col {col}, row {row_below_idx}: below '{char_below}' p={} h={} doesn't fill cell (need p=0, p+h={})",
						candle_below.placement, candle_below.height, CANDLE_SIZE
					));
				}
			}
		}

		if !errors.is_empty() {
			panic!(
				"Multi-row candle continuity errors ({} errors in {} adjacent pairs):\n{}",
				errors.len(),
				adjacent_pairs,
				errors.join("\n")
			);
		}
	}
}
