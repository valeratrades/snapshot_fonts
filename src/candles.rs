use std::path::Path;

use crate::fill_levels::{LEVELS, PUA_START};

/// Candle size (max height = 11, giving 12 placement positions 0..=11)
pub const CANDLE_SIZE: u8 = 11;
/// Total number of candle glyphs (calculated by calc_candles script: 52416 + 1 empty = 52417)
pub const CANDLE_GLYPH_COUNT: u32 = 52417;
/// Start of candle font in PUA (after FillLevels: 251*251 = 63001 glyphs)
pub const CANDLE_PUA_START: u32 = PUA_START + (LEVELS as u32 * LEVELS as u32);

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
	let glyph_script = format!(
		r#"
SIZE = {size}
PUA_START = {pua_start}

# Wick is middle third
WICK_LEFT = GLYPH_WIDTH // 3
WICK_RIGHT = 2 * GLYPH_WIDTH // 3

# Body width is full glyph
BODY_LEFT = 0
BODY_RIGHT = GLYPH_WIDTH

# Height of one level in font units
LEVEL_HEIGHT = LINE_HEIGHT // (SIZE + 1)

# Doji body height (1/44 of char height)
DOJI_HEIGHT = LINE_HEIGHT // 44

char_code = PUA_START

# Index 0: empty candle
glyph = font.createChar(char_code)
glyph.width = GLYPH_WIDTH
char_code += 1

# For each height h (0..=SIZE)
for h in range(SIZE + 1):
    n_borders = h + 1

    # For each placement p (0..=SIZE) - where candle sits within the character
    for p in range(SIZE + 1):
        # Calculate base y position for this placement
        # placement 0 = candle at bottom, placement SIZE = candle at top
        base_y = -HHEA_DESCENT + p * LEVEL_HEIGHT

        # For each body configuration (matching calc_candles iteration order)
        for body_start in range(n_borders):
            for body_size in range(n_borders - body_start):
                # Calculate wick options
                options_wick_above = 1 + body_start
                options_wick_below = n_borders - (body_start + body_size)

                # Generate all wick combinations
                for wick_above in range(options_wick_above):
                    for wick_below in range(options_wick_below):
                        glyph = font.createChar(char_code)
                        glyph.width = GLYPH_WIDTH
                        pen = glyph.glyphPen()

                        # Calculate y coordinates
                        # Candle spans from base_y to base_y + h * LEVEL_HEIGHT
                        candle_top = base_y + h * LEVEL_HEIGHT
                        candle_bottom = base_y

                        # Body position within candle
                        body_top_y = candle_top - body_start * LEVEL_HEIGHT
                        if body_size == 0:
                            # Doji: thin body
                            body_bottom_y = body_top_y - DOJI_HEIGHT
                        else:
                            body_bottom_y = body_top_y - body_size * LEVEL_HEIGHT

                        # Wick extends above and below body
                        wick_top_y = body_top_y + wick_above * LEVEL_HEIGHT
                        wick_bottom_y = body_bottom_y - wick_below * LEVEL_HEIGHT

                        # Clamp to candle bounds
                        wick_top_y = min(wick_top_y, candle_top)
                        wick_bottom_y = max(wick_bottom_y, candle_bottom)

                        # Draw wick (thin vertical line in center)
                        if wick_top_y > wick_bottom_y:
                            pen.moveTo((WICK_LEFT, wick_bottom_y))
                            pen.lineTo((WICK_RIGHT, wick_bottom_y))
                            pen.lineTo((WICK_RIGHT, wick_top_y))
                            pen.lineTo((WICK_LEFT, wick_top_y))
                            pen.closePath()

                        # Draw body (wider rectangle)
                        if body_top_y > body_bottom_y:
                            pen.moveTo((BODY_LEFT, body_bottom_y))
                            pen.lineTo((BODY_RIGHT, body_bottom_y))
                            pen.lineTo((BODY_RIGHT, body_top_y))
                            pen.lineTo((BODY_LEFT, body_top_y))
                            pen.closePath()

                        pen = None
                        char_code += 1
"#,
		size = CANDLE_SIZE,
		pua_start = CANDLE_PUA_START,
	);

	crate::fontforge::generate_font("Candles", output, &glyph_script)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_candle_encode_decode_roundtrip() {
		// Test a few representative candles
		let test_cases = [
			Candle::empty(),
			Candle::new(0, 5, 0, 0),   // placement 0, height 5, doji at top
			Candle::new(3, 5, 2, 0),   // placement 3, height 5, doji in middle
			Candle::new(6, 5, 5, 0),   // placement 6, height 5, doji at bottom
			Candle::new(0, 11, 0, 11), // full body, no wick room
			Candle::new(5, 11, 2, 5),  // body in middle
			Candle::new(11, 11, 0, 0), // max placement, max height, doji at top
		];

		for candle in test_cases {
			let encoded = encode_candle(candle);
			let decoded = decode_candle(encoded);
			assert_eq!(candle, decoded, "roundtrip failed for {:?}", candle);
		}
	}

	#[test]
	fn test_total_glyph_count() {
		let size = CANDLE_SIZE as u32;
		let placement_options = size + 1; // 12

		let mut total: u32 = 1; // +1 for empty
		for h in 0..=size {
			total += candle_count_for_height(h, size) * placement_options;
		}
		assert_eq!(total, CANDLE_GLYPH_COUNT, "total glyph count mismatch");
	}
}
