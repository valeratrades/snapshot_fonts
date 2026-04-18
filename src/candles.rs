use std::path::Path;

use crate::fontforge::{GLYPH_WIDTH, HHEA_DESCENT, LINE_HEIGHT};

/// Candle size (max height = 11, giving 12 placement positions 0..=11)
pub const CANDLE_SIZE: u8 = 11;
/// Number of naked wick glyphs:
/// - high_offset=0, height=1..=11 (wick comes from above): 11 options
/// - high_offset=1..=10, height fills to bottom (wick goes below): 10 options
///
/// Total: 21
pub const NAKED_WICK_COUNT: u32 = 21;
/// Total number of candle glyphs:
/// - 1 empty
/// - 52416 regular (with body)
/// - 21 naked wick (wick only, connected to outside)
///
/// = 52438
pub const CANDLE_GLYPH_COUNT: u32 = 52438;
/// Start of candle font in PUA-B (Plane 16): U+100000–U+10FFFD
pub const CANDLE_PUA_START: u32 = 0x100000;
/// Sentinel value indicating naked wick (no body in this cell)
pub const NAKED_WICK: i8 = -1;

// Derived constants for glyph geometry
const WICK_LEFT: i32 = GLYPH_WIDTH / 3;
const WICK_RIGHT: i32 = 2 * GLYPH_WIDTH / 3;
const BODY_LEFT: i32 = 0;
const BODY_RIGHT: i32 = GLYPH_WIDTH;
const LEVEL_HEIGHT: i32 = LINE_HEIGHT / CANDLE_SIZE as i32;
const DOJI_HEIGHT: i32 = LINE_HEIGHT / 32;

use v_utils::trades::Ohlc;

const DEFAULT_WIDTH: usize = 90;
const DEFAULT_HEIGHT: usize = 12;
/// Single candlestick glyph within a character cell.
///
/// When `body_offset` and `body_size` are both `NAKED_WICK` (-1), the candle has no body
/// in this cell (used when the body is entirely in an adjacent cell).
///
/// # Geometry (offsets from top of char cell)
/// - Wick top (HIGH): at `high_offset`
/// - Top wick: from `high_offset` to `high_offset + body_offset`
/// - Body top: at `high_offset + body_offset`
/// - Body bottom: at `high_offset + body_offset + body_size`
/// - Bottom wick: from `high_offset + body_offset + body_size` to `high_offset + height`
/// - Wick bottom (LOW): at `high_offset + height`
///
/// Rendering: draw wick from `high_offset` through `high_offset + height`, then overlay body.
#[derive(Clone, Copy, Debug, Eq, PartialEq, derive_new::new)]
pub struct Candle {
	/// Offset from top of char cell to wick high (0-11).
	pub high_offset: u8,
	/// Total wick span from high to low (0-11). Candle extends to `high_offset + height`.
	pub height: u8,
	/// Offset from `high_offset` to body top (0 to height), or `NAKED_WICK` (-1).
	/// When 0, body starts at the wick top (no top wick).
	pub body_offset: i8,
	/// Body size (0 = doji, drawn as thin line), or `NAKED_WICK` (-1).
	pub body_size: i8,
}
impl Candle {
	/// Create a naked wick candle (wick only, no body in this cell)
	pub fn naked_wick(high_offset: u8, height: u8) -> Self {
		Candle {
			high_offset,
			height,
			body_offset: NAKED_WICK,
			body_size: NAKED_WICK,
		}
	}

	/// Create empty candle (codepoint 0 in candle range)
	pub fn empty() -> Self {
		Candle {
			high_offset: 0,
			height: 0,
			body_offset: 0,
			body_size: 0,
		}
	}

	/// Check if this is the empty candle
	pub fn is_empty(&self) -> bool {
		// Empty candle is index 0, which is height=0, high_offset=0, body=0
		self.height == 0 && self.high_offset == 0 && self.body_offset == 0 && self.body_size == 0
	}

	/// Check if this is a naked wick (wick only, no body)
	pub fn is_naked_wick(&self) -> bool {
		self.body_offset == NAKED_WICK && self.body_size == NAKED_WICK
	}

	/// Validate candle parameters and return error message if invalid
	pub fn validate(&self) -> Result<(), String> {
		let size = CANDLE_SIZE;

		// Empty candle is always valid
		if self.is_empty() {
			return Ok(());
		}

		// Check high_offset bounds
		if self.high_offset > size {
			return Err(format!("high_offset {} out of range [0, {size}]", self.high_offset));
		}

		// Check height bounds
		if self.height > size {
			return Err(format!("height {} out of range [0, {size}]", self.height));
		}

		// Check that candle fits in cell
		if self.high_offset + self.height > size {
			return Err(format!(
				"candle extends beyond cell: high_offset({}) + height({}) = {} > {size}",
				self.high_offset,
				self.height,
				self.high_offset + self.height
			));
		}

		// Naked wick validation: must connect to outside
		// Either high_offset=0 (connects above) or high_offset+height=size (connects below)
		if self.is_naked_wick() {
			let connects_above = self.high_offset == 0;
			let connects_below = self.high_offset + self.height == size;
			if !connects_above && !connects_below {
				return Err(format!(
					"naked wick must connect to outside: high_offset={}, height={}, need high_offset=0 OR high_offset+height={size}",
					self.high_offset, self.height
				));
			}
			if self.height == 0 {
				return Err("naked wick cannot have height=0".to_string());
			}
			return Ok(());
		}

		// Body parameter validation (non-naked wick)
		if self.body_offset < 0 || self.body_offset > self.height as i8 {
			return Err(format!("body_offset {} out of range [0, height={}]", self.body_offset, self.height));
		}

		if self.body_size < 0 {
			return Err(format!("body_size {} cannot be negative (use NAKED_WICK for both body params)", self.body_size));
		}

		if self.body_offset + self.body_size > self.height as i8 {
			return Err(format!(
				"body extends beyond wick: body_offset({}) + body_size({}) = {} > height({})",
				self.body_offset,
				self.body_size,
				self.body_offset + self.body_size,
				self.height
			));
		}

		Ok(())
	}
}

/// Candlestick chart snapshot
#[derive(bon::Builder, Clone, Debug)]
pub struct SnapshotCandles {
	ohlcs: Vec<Ohlc>,
	#[builder(default = DEFAULT_WIDTH)]
	width: usize,
	#[builder(default = DEFAULT_HEIGHT)]
	height: usize,
}
impl SnapshotCandles {
	/// Create from price series - step size is calculated from width
	pub fn from_prices<T: Into<f64> + Copy>(prices: &[T]) -> Self {
		let prices: Vec<f64> = prices.iter().map(|p| (*p).into()).collect();
		let step = (prices.len() / DEFAULT_WIDTH).max(1);
		let ohlcs = v_utils::trades::mock_p_to_ohlc(&prices, step);
		Self::builder().ohlcs(ohlcs).build()
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
		// Note: levels increase upward (0 = min price, total_levels-1 = max price)
		// But placement is offset from TOP of cell (0 = top of cell)
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

				// Wick bounds within this row (in row-local coordinates, 0 = row_bottom)
				// Extend to boundary only on the side that actually extends beyond
				// We use "exclusive" bounds: local_high is one past the top level
				let local_low = if extends_below { 0 } else { low - row_bottom };
				let local_high = if extends_above { levels_per_row } else { high - row_bottom + 1 };

				// Convert to new coordinate system where 0 = top of cell
				// high_offset = offset from top of cell to wick top (HIGH point)
				let high_offset = levels_per_row - local_high;

				// height = wick span (from high to low)
				let height = local_high - local_low;

				// Body bounds (in global levels)
				let body_top_global = open.max(close);
				let body_bottom_global = open.min(close);

				// Check if body intersects this row
				let body_intersects = !(body_top_global < row_bottom || body_bottom_global > row_top);

				let candle = if !body_intersects {
					// Body doesn't intersect this row - naked wick
					Candle::naked_wick(high_offset as u8, height as u8)
				} else {
					// Body intersects - clamp to row boundaries
					let body_extends_below = body_bottom_global < row_bottom;
					let body_extends_above = body_top_global > row_top;

					// Extend to boundary only on the side that actually extends beyond
					let local_body_bottom = if body_extends_below { 0 } else { body_bottom_global - row_bottom };
					let local_body_top = if body_extends_above { levels_per_row } else { body_top_global - row_bottom + 1 };

					// body_offset = offset from high_offset to body top
					// local_high is the exclusive top of wick, local_body_top is exclusive top of body
					let bo = local_high - local_body_top;
					// body_size = body span (0 for doji when open == close)
					let bsz = if body_top_global == body_bottom_global { 0 } else { local_body_top - local_body_bottom };
					Candle::new(high_offset as u8, height as u8, bo as i8, bsz as i8)
				};
				row_chars.push(encode_candle(candle));
			}
			rows.push(row_chars.iter().collect());
		}

		rows.join("\n")
	}
}

/// Decode a Unicode codepoint back to a Candle
pub fn decode_candle(c: char) -> Candle {
	let index = c as u32 - CANDLE_PUA_START;

	// Index 0 is empty
	if index == 0 {
		return Candle::empty();
	}

	let size = CANDLE_SIZE as u32;
	let regular_count = 52416u32;

	// Check if this is a naked wick glyph (after regular glyphs)
	if index > regular_count {
		let remaining = index - 1 - regular_count;

		// Naked wick layout:
		// - First 11 (indices 0-10): high_offset=0, height=1..=11
		// - Next 10 (indices 11-20): high_offset=1..=10, height fills to bottom
		if remaining < 11 {
			// high_offset=0 case
			let height = remaining + 1;
			return Candle::naked_wick(0, height as u8);
		} else {
			// high_offset>0 case
			let high_offset = remaining - 11 + 1;
			let height = size - high_offset; // fills to bottom
			return Candle::naked_wick(high_offset as u8, height as u8);
		}
	}

	// Regular candle decoding
	let mut remaining = index - 1; // Subtract 1 for empty
	let high_offset_options = size + 1;

	// Find height
	let mut height: u32 = 0;
	while height <= size {
		let count = candle_count_for_height(height, size) * high_offset_options;
		if remaining < count {
			break;
		}
		remaining -= count;
		height += 1;
	}

	if height > size {
		return Candle::empty();
	}

	// Find high_offset within this height
	let count_per_high_offset = candle_count_for_height(height, size);
	let high_offset = remaining / count_per_high_offset;
	remaining %= count_per_high_offset;

	// Find body_offset and body_size
	let n_borders = height + 1;
	let mut body_offset: u32 = 0;
	while body_offset < n_borders {
		let mut count_for_bs: u32 = 0;
		for bsz in 0..(n_borders - body_offset) {
			count_for_bs += wick_options(body_offset, bsz, n_borders);
		}
		if remaining < count_for_bs {
			break;
		}
		remaining -= count_for_bs;
		body_offset += 1;
	}

	let mut body_size: u32 = 0;
	while body_offset + body_size < n_borders {
		let count = wick_options(body_offset, body_size, n_borders);
		if remaining < count {
			break;
		}
		remaining -= count;
		body_size += 1;
	}

	Candle::new(high_offset as u8, height as u8, body_offset as i8, body_size as i8)
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
/// - If body_size == 0 (doji), body is drawn as 1/32 of char height
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
		lines.push(format!("g=font.createChar({});g.width={GLYPH_WIDTH}", self.char_code));

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
///
/// Coordinate system:
/// - Font Y=0 is at bottom of cell, increasing upward
/// - Cell bottom: -HHEA_DESCENT, Cell top: -HHEA_DESCENT + LINE_HEIGHT
/// - `placement` is offset from TOP of cell (0 = at top, 11 = at bottom)
/// - Candle extends DOWN from placement, so:
///   - Wick top (HIGH) at: cell_top - placement * LEVEL_HEIGHT
///   - Wick bottom (LOW) at: wick_top - height * LEVEL_HEIGHT
/// - Body is within the wick:
///   - Body top at: wick_top - body_start * LEVEL_HEIGHT
///   - Body bottom at: body_top - body_size * LEVEL_HEIGHT (or DOJI_HEIGHT if body_size=0)
fn generate_all_glyphs() -> Vec<CandleGlyph> {
	let mut glyphs = Vec::with_capacity(CANDLE_GLYPH_COUNT as usize);
	let mut char_code = CANDLE_PUA_START;
	let size = CANDLE_SIZE as i32;
	let cell_top = -HHEA_DESCENT + LINE_HEIGHT;

	// Index 0: empty candle
	glyphs.push(CandleGlyph { char_code, wick: None, body: None });
	char_code += 1;

	// For each height h (0..=SIZE)
	for h in 0..=size {
		let n_borders = h + 1;

		// For each placement p (0..=SIZE)
		for p in 0..=size {
			// Wick top (HIGH) - placement levels down from cell top
			let wick_top_y = cell_top - p * LEVEL_HEIGHT;
			// Wick bottom (LOW) - height levels down from wick top
			let wick_bottom_y = wick_top_y - h * LEVEL_HEIGHT;

			// For each body configuration
			for body_start in 0..n_borders {
				for body_size in 0..(n_borders - body_start) {
					let options_wick_above = 1 + body_start;
					let options_wick_below = n_borders - (body_start + body_size);

					// Generate all wick combinations (these are for the encoding, not geometry)
					// We generate glyphs for each wick option but the actual geometry
					// is determined by placement/height/body_start/body_size only
					for _wick_above in 0..options_wick_above {
						for _wick_below in 0..options_wick_below {
							// Body top - body_start levels down from wick top
							let body_top_y = wick_top_y - body_start * LEVEL_HEIGHT;
							// Body bottom - body_size levels down from body top (or DOJI_HEIGHT for doji)
							let body_bottom_y = if body_size == 0 { body_top_y - DOJI_HEIGHT } else { body_top_y - body_size * LEVEL_HEIGHT };

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

	// Naked wick glyphs (wick only, body is outside this cell)
	// Must connect to outside: either starts at top (high_offset=0) or ends at bottom

	// high_offset=0, height=1..=11 (wick comes from above)
	for h in 1..=size {
		let wick_top_y = cell_top; // p=0
		let wick_bottom_y = wick_top_y - h * LEVEL_HEIGHT;

		let wick = Rect {
			left: WICK_LEFT,
			right: WICK_RIGHT,
			bottom: wick_bottom_y,
			top: wick_top_y,
		};

		glyphs.push(CandleGlyph {
			char_code,
			wick: wick.is_valid().then_some(wick),
			body: None,
		});
		char_code += 1;
	}

	// high_offset=1..=10, height fills to bottom (wick goes below)
	for p in 1..size {
		let h = size - p; // height to fill to bottom
		let wick_top_y = cell_top - p * LEVEL_HEIGHT;
		let wick_bottom_y = wick_top_y - h * LEVEL_HEIGHT;

		let wick = Rect {
			left: WICK_LEFT,
			right: WICK_RIGHT,
			bottom: wick_bottom_y,
			top: wick_top_y,
		};

		glyphs.push(CandleGlyph {
			char_code,
			wick: wick.is_valid().then_some(wick),
			body: None,
		});
		char_code += 1;
	}

	glyphs
}

// ============================================================================
// Chart rendering
// ============================================================================

/// Encode a candle into a Unicode codepoint in PUA
/// Layout:
/// - Index 0: empty candle
/// - Indices 1..52417: regular candles (with body)
/// - Indices 52417..52483: naked wick candles (no body)
pub(crate) fn encode_candle(candle: Candle) -> char {
	// Validate before encoding
	if let Err(e) = candle.validate() {
		panic!("Invalid candle: {e}");
	}

	// Index 0 is empty
	if candle.is_empty() {
		return char::from_u32(CANDLE_PUA_START).expect("valid codepoint");
	}

	let size = CANDLE_SIZE as u32;

	// Handle naked wick separately
	if candle.is_naked_wick() {
		// Naked wick index starts after all regular glyphs
		let regular_count = 52416u32; // Pre-calculated
		let mut index = 1 + regular_count;

		let h = candle.height as u32;
		let p = candle.high_offset as u32;

		// Naked wick layout:
		// - First 11: high_offset=0, height=1..=11
		// - Next 10: high_offset=1..=10, height fills to bottom (11-high_offset)
		if p == 0 {
			// high_offset=0 case: index by height (1-indexed)
			index += h - 1;
		} else {
			// high_offset>0 case: must fill to bottom
			// These come after the 11 high_offset=0 glyphs
			index += 11 + (p - 1);
		}

		let code = CANDLE_PUA_START + index;
		return char::from_u32(code).expect("valid codepoint");
	}

	// Regular candle encoding
	let mut index: u32 = 1; // Start at 1 (0 is empty)
	let high_offset_options = size + 1; // 12

	// Add all glyphs from previous heights
	for h in 0..candle.height as u32 {
		index += candle_count_for_height(h, size) * high_offset_options;
	}

	// Add glyphs from previous high_offsets within this height
	let h = candle.height as u32;
	index += candle.high_offset as u32 * candle_count_for_height(h, size);

	// Add glyphs from previous body configs within this height/high_offset
	// Iterate body_offset from 0
	let n_borders = h + 1;
	for bs in 0..candle.body_offset as u32 {
		for bsz in 0..(n_borders - bs) {
			index += wick_options(bs, bsz, n_borders);
		}
	}

	// Add glyphs from previous body_size within this body_offset
	for bsz in 0..candle.body_size as u32 {
		index += wick_options(candle.body_offset as u32, bsz, n_borders);
	}

	// We use first wick option (wick_above=0, wick_below=0)
	let code = CANDLE_PUA_START + index;
	char::from_u32(code).expect("valid codepoint")
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
/// open_offset_i = body_offset (distance from top)
/// close_from_open_j = body_size
fn wick_options(open_offset_i: u32, close_from_open_j: u32, n_borders: u32) -> u32 {
	let options_wick_above = 1 + open_offset_i;
	let options_wick_below = n_borders - (open_offset_i + close_from_open_j);
	options_wick_above * options_wick_below
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
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀈞􁮤􀣱􉅏􌳕􀈞􌳕􀀀􀈩􌳔􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀒽􀒬􀀀􈴒􀄆􀄆􀁎􀗴􂕍􉄧􁦨􌳄􅾜􀁁􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􌳐􈴒􀯧􀝷􀀏􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􃲋􀀀􀀀􌳕􀀀􀀀􀀀􀀀􁮤􈻈􀷟􆏷
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀈱􄈡􁟊􃳤􀊯􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􁘃􀺍􀱬􀠒􀤎􀀀􀃕􅸚􌳁􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀣱􀶂􀯣􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀯬􁡒􀊦􀀀􀀀􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􈴒􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀈱􁯸􌳔􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􌳔􂰇􀤎􀀀􀀀􀀀􀀀􀀀􀀀􀃕􀽺􄑌􁭩􁂟􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􄈡􀊯􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀃕􁗌􀊦􀗸􂬚􀈩􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀒬􁀯􁀫􀁁􀀀􃵏􀄆􀊭􁜱􂬚􌳔􀈞􁀯􀥂􃲋􀀀􀀀􀄁􀗽􁫴􀃕􈴒􆈃􀀀􀀀􀀀􄎼􀞔􀯬􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􀒨􀒙􀀀􀃚􀒙􌳕􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􌳔􅸚􀀀􀀀􀀀􁖾􀄆􈸹􀀀􀀀􀀀􁮤􁫫􁡡􀍎􀊭􀀀􌳁􁗇􁗌􀀀􀀀􀀀􀀀􅷿􁙨􀊭􀀀􀄁􀀀􀀀􀀀􀀀􀀀􀊭􀊯􀀀􅸚􀃐􀓈􁫴􀗻􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􌳃􂔹􀷥􀗽􀗸􂕬􅼍􁯸􀒗􌳕􀀀􀀀􀀀􀀀􀀀􈴒􀀀􀀀􀀀􀀀􀀀􀀀􅸕􄂉􀃕􆒍􃲋􌳅􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􌳋􀗯􀀏􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀏􌳃􅸑􁬫􀁁􌳔􀀀􄈡􀄄􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􁗃􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􌳁􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
		􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀊯􀗽􀊡􆋛􂕍􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀
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
					format!("row{i}:p={},h={},bo={},bsz={}", candle.high_offset, candle.height, candle.body_offset, candle.body_size)
				}
			})
			.collect();

		// for ref, the translated candle has:
		// high_level = 34
		// low_level = 21
		// open_level = 34
		// close_level = 34
		assert_snapshot!(problematic_col.iter().collect::<String>(), @"􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀀀􀃐􌳋􌳁􀀀"); // visual check; this succeeding means nothing, as we're yet to have a correct representation, that I would've then fixed here.
		assert_snapshot!(decoded.join("\n"), @r"
		row0:empty
		row1:empty
		row2:empty
		row3:empty
		row4:empty
		row5:empty
		row6:empty
		row7:empty
		row8:p=9,h=2,bo=0,bsz=0
		row9:p=0,h=11,bo=-1,bsz=-1
		row10:p=0,h=1,bo=-1,bsz=-1
		row11:empty
		"); // this succeeding means nothing too

		let candle8 = decode_candle(problematic_col[8]);
		assert_eq!(candle8.body_offset, 0);
		assert_eq!(candle8.body_size, 0);

		let candle9 = decode_candle(problematic_col[9]);
		assert_eq!(candle9, Candle::new(0, CANDLE_SIZE, NAKED_WICK, NAKED_WICK));

		let candle10 = decode_candle(problematic_col[10]);
		assert_eq!(candle10.high_offset, 0);
		assert_eq!(candle10.body_offset, NAKED_WICK);
		assert_eq!(candle10.body_size, NAKED_WICK);
	}

	/// Test that multi-row candles are properly connected at row boundaries.
	///
	/// For two adjacent rows (above and below) with candles in the same column:
	/// - Above candle must extend to bottom of its cell: high_offset + height == CANDLE_SIZE
	/// - Below candle must extend to top of its cell: high_offset == 0
	///
	/// This ensures continuous vertical joining without gaps.
	#[test]
	fn test_multirow_candle_continuity() {
		let chart = test_chart();
		let rows: Vec<&str> = chart.lines().collect();
		let height = rows.len();

		if height < 2 {
			return;
		}

		let decoded: Vec<Vec<(char, Candle)>> = rows.iter().map(|row| row.chars().map(|c| (c, decode_candle(c))).collect()).collect();

		let width = decoded[0].len();
		let mut errors: Vec<String> = Vec::new();

		#[allow(clippy::needless_range_loop)] //Q: should I make this global? Because eg here it makes logic more apparent (like O(n^2) nature)
		for col in 0..width {
			for row_above_idx in 0..(height - 1) {
				let row_below_idx = row_above_idx + 1;
				let (char_above, candle_above) = decoded[row_above_idx][col];
				let (char_below, candle_below) = decoded[row_below_idx][col];

				// Skip if either cell is empty - no continuity to check
				if candle_above.is_empty() || candle_below.is_empty() {
					continue;
				}

				// Both cells have candles - verify they connect at the boundary
				let above_touches_bottom = candle_above.high_offset + candle_above.height == CANDLE_SIZE;
				let below_touches_top = candle_below.high_offset == 0;

				if !above_touches_bottom {
					errors.push(format!(
						"Col {col}, rows {row_above_idx}-{row_below_idx}: above candle '{char_above}' (p={}, h={}) doesn't reach bottom (p+h={}, need {CANDLE_SIZE})",
						candle_above.high_offset,
						candle_above.height,
						candle_above.high_offset + candle_above.height
					));
				}

				if !below_touches_top {
					errors.push(format!(
						"Col {col}, rows {row_above_idx}-{row_below_idx}: below candle '{char_below}' (p={}, h={}) doesn't start at top (p={}, need 0)",
						candle_below.high_offset, candle_below.height, candle_below.high_offset
					));
				}
			}
		}

		assert!(errors.is_empty(), "Multi-row candle continuity errors ({} errors):\n{}", errors.len(), errors.join("\n"));
	}
}
