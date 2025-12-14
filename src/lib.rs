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

	let mut child = Command::new("nix-shell")
		.args(["-p", "fontforge", "--run", "fontforge -lang=py -script /dev/stdin"])
		.stdin(Stdio::piped())
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()?;

	child.stdin.take().unwrap().write_all(script.as_bytes())?;

	let status = child.wait()?;
	if status.success() {
		Ok(())
	} else {
		Err(std::io::Error::new(std::io::ErrorKind::Other, "fontforge failed"))
	}
}

/// Plot data helper for scaling values to 0-250 range
struct PlotScaler {
	scale: f64,
	offset: f64,
}

impl PlotScaler {
	fn new(min_val: f64, max_val: f64) -> Self {
		let data_range = max_val - min_val;
		let scale = 250.0 / data_range;
		let offset = min_val;
		PlotScaler { scale, offset }
	}

	fn to_level(&self, val: f64) -> u8 {
		((val - self.offset) * self.scale).clamp(0.0, 250.0) as u8
	}
}

/// Snapshot builder for plotting data using the FillLevels font
#[derive(Clone, Debug)]
pub struct Snapshot {
	values: Vec<f64>,
	secondary: Option<Vec<Option<f64>>>,
	width: usize,
}

impl Snapshot {
	/// Create a new snapshot from a slice of values
	pub fn new<T: Into<f64> + Copy>(values: &[T]) -> Self {
		Snapshot {
			values: values.iter().map(|x| (*x).into()).collect(),
			secondary: None,
			width: 90,
		}
	}

	/// Add a secondary data series (plotted as the right bar)
	pub fn secondary<T: Into<f64> + Copy>(mut self, values: &[T]) -> Self {
		self.secondary = Some(values.iter().map(|x| Some((*x).into())).collect());
		self
	}

	/// Add a secondary data series with optional values
	pub fn secondary_optional<T: Into<f64> + Copy>(mut self, values: Vec<Option<T>>) -> Self {
		self.secondary = Some(values.into_iter().map(|x| x.map(|v| v.into())).collect());
		self
	}

	/// Set the output width in characters (default: 90)
	pub fn width(mut self, width: usize) -> Self {
		self.width = width;
		self
	}

	/// Render to a string using FillLevels font encoding
	/// Each character encodes two bars: left = primary data, right = secondary data (or same as left if no secondary)
	pub fn render(&self) -> String {
		if self.values.is_empty() {
			return String::new();
		}

		let min_val = self.values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
		let max_val = self.values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

		if (max_val - min_val).abs() < f64::EPSILON {
			// All values the same - render as mid-height bars
			return (0..self.width).map(|_| encode_bars(125, 125)).collect();
		}

		let scaler = PlotScaler::new(min_val, max_val);

		let secondary_scaler = self
			.secondary
			.as_ref()
			.map(|sec| {
				let sec_vals: Vec<f64> = sec.iter().filter_map(|x| *x).collect();
				if sec_vals.is_empty() {
					return None;
				}
				let sec_min = sec_vals.iter().fold(f64::INFINITY, |a, &b| a.min(b));
				let sec_max = sec_vals.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
				if (sec_max - sec_min).abs() < f64::EPSILON {
					None
				} else {
					Some(PlotScaler::new(sec_min, sec_max))
				}
			})
			.flatten();

		(0..self.width)
			.map(|j| {
				let idx = (j as f64 * self.values.len() as f64 / self.width as f64) as usize;
				let idx = idx.min(self.values.len() - 1);

				let left = scaler.to_level(self.values[idx]);

				let right = match (&self.secondary, &secondary_scaler) {
					(Some(sec), Some(sec_scaler)) => {
						let sec_idx = (j as f64 * sec.len() as f64 / self.width as f64) as usize;
						let sec_idx = sec_idx.min(sec.len() - 1);
						match sec[sec_idx] {
							Some(v) => sec_scaler.to_level(v),
							None => 0,
						}
					}
					(Some(sec), None) => {
						// Secondary exists but has no range - use mid value or 0 for None
						let sec_idx = (j as f64 * sec.len() as f64 / self.width as f64) as usize;
						let sec_idx = sec_idx.min(sec.len() - 1);
						match sec[sec_idx] {
							Some(_) => 125,
							None => 0,
						}
					}
					(None, _) => left, // No secondary - mirror primary
				};

				encode_bars(left, right)
			})
			.collect()
	}

	/// Render with value annotations (min/max labels)
	pub fn render_annotated(&self) -> String {
		if self.values.is_empty() {
			return String::new();
		}

		let min_val = self.values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
		let max_val = self.values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

		let plot = self.render();
		format!("{:.2}\n{}\n{:.2}", max_val, plot, min_val)
	}
}

#[cfg(test)]
mod tests {
	use insta::assert_snapshot;
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
	fn test_snapshot_linear_ramp() {
		let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
		let rendered = Snapshot::new(&data).width(50).render();
		assert_snapshot!(rendered, @"\0Ӭ\u{9d8}ໄᎰᢜᶈ≴❠ⱌㄸ㘤㬐㿼䓨䧔什厬墘嶄捬桘浄爰眜簈胴藠諌辸钤馐鹼ꍨꡔ굀눬뜘밄샰웘쯄킰햜\u{e288}\u{e774}\u{ec60}\u{f14c}\u{f638}ﬤ");
	}

	#[test]
	fn test_snapshot_opposing() {
		let primary: Vec<f64> = (0..100).map(|i| i as f64).collect();
		let secondary: Vec<f64> = (0..100).map(|i| (99 - i) as f64).collect();
		let rendered = Snapshot::new(&primary).secondary(&secondary).width(50).render();
		assert_snapshot!(rendered, @"ùכઽྟᒁᥣṅ⌧⠉Ⳬ㇍㚯㮑䁳䕕䨷伙叻壝嶿掛桽浟牁眣簅胧藉誫辍鑯饑鸳ꌕꟷ곙놻뚝뭿쁡옽쬟퀁퓣隷");
	}

	#[test]
	fn test_snapshot_random_walk() {
		let data = laplace_random_walk(100.0, 500, 0.1, 0.0, Some(42));
		let rendered = Snapshot::new(&data).width(80).render();
		assert_snapshot!(rendered, @"諌裔稐扰眜汈苬苬縀稐敤縀嶄庀妔䳈䳈ㄸ䏬ⱌ⡜⭐〼⁼ר≴䋰笌縀鶀봀톬풠쓠쳀ꑤꍨ똜Ꙝ庀焴完䗤杜敤浄验瀸障鎨ꁴ颔넰딠댨먌긼뀴馐馐ꩌ싨춼쾴샰뀴ꁴ鲄ꁴ鹼넰렔봀ꭈ뷼");
	}

	#[test]
	fn test_snapshot_random_walk_annotated() {
		let data = laplace_random_walk(100.0, 500, 0.1, 0.0, Some(42));
		let rendered = Snapshot::new(&data).width(80).render_annotated();
		assert_snapshot!(rendered, @r"
		100.98
		諌裔稐扰眜汈苬苬縀稐敤縀嶄庀妔䳈䳈ㄸ䏬ⱌ⡜⭐〼⁼ר≴䋰笌縀鶀봀톬풠쓠쳀ꑤꍨ똜Ꙝ庀焴完䗤杜敤浄验瀸障鎨ꁴ颔넰딠댨먌긼뀴馐馐ꩌ싨춼쾴샰뀴ꁴ鲄ꁴ鹼넰렔봀ꭈ뷼
		98.73
		");
	}

	#[test]
	fn test_snapshot_with_secondary_optional() {
		let primary = laplace_random_walk(100.0, 200, 0.1, 0.0, Some(42));
		let secondary: Vec<Option<f64>> = (0..200).map(|i| if i % 3 == 0 { None } else { Some((i as f64).sin() * 50.0 + 100.0) }).collect();
		let rendered = Snapshot::new(&primary).secondary_optional(secondary).width(60).render();
		assert_snapshot!(rendered, @"雾鯥锈鑆觵臅禈碬腍碙碙繻褣蹀褶覠睑蒘玲粅葝择樌峭柮尥愚堾呒尪厔園㻢䘔䄦ㆠṥ⅖⠳⅞Ⰽ㐑⁗⍌ᖐעີⴚ䐿厧廙浽鉹ꑂ黖봻췦");
	}
}
