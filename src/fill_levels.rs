use std::path::Path;

pub const LEVELS: u16 = 251;
/// Start of fill_levels glyphs in PUA-A (Plane 15), positioned to end at U+FFFFD
pub const PUA_START: u32 = 0xf09e5;

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

#[cfg(test)]
mod tests {
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
}
