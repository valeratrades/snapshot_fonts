
> [!WARNING]
> Uses Unicode Private Use Areas: \
> - FillLevels: PUA-A (Plane 15) U+F0000U+FFFFD \
> - Candles: PUA-B (Plane 16) U+100000U+10FFFD \
>  \
> These ranges may conflict with other custom fonts using PUA.
# snapshot_fonts
![Minimum Supported Rust Version](https://img.shields.io/badge/nightly-1.93+-ab6000.svg)
[<img alt="crates.io" src="https://img.shields.io/crates/v/snapshot_fonts.svg?color=fc8d62&logo=rust" height="20" style=flat-square>](https://crates.io/crates/snapshot_fonts)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs&style=flat-square" height="20">](https://docs.rs/snapshot_fonts)
![Lines Of Code](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/snapshot_fonts-loc.json)
<br>
[<img alt="ci errors" src="https://img.shields.io/github/actions/workflow/status/valeratrades/snapshot_fonts/errors.yml?branch=master&style=for-the-badge&style=flat-square&label=errors&labelColor=420d09" height="20">](https://github.com/valeratrades/snapshot_fonts/actions?query=branch%3Amaster) <!--NB: Won't find it if repo is private-->
[<img alt="ci warnings" src="https://img.shields.io/github/actions/workflow/status/valeratrades/snapshot_fonts/warnings.yml?branch=master&style=for-the-badge&style=flat-square&label=warnings&labelColor=d16002" height="20">](https://github.com/valeratrades/snapshot_fonts/actions?query=branch%3Amaster) <!--NB: Won't find it if repo is private-->

Monospace fonts for terminal-based data visualization. Encode chart data directly in Unicode text.

- **FillLevels**: 251×251 fill level combinations for sparklines/histograms
- **Candles**: 52k candlestick glyphs for financial charts
<!-- markdownlint-disable -->
<details>
<summary>
<h3>Installation</h3>
</summary>

```sh
cargo install snapshot_fonts
snapshot_fonts generate -o ~/.local/share/fonts/
fc-cache -fv
```

</details>
<!-- markdownlint-restore -->

## Usage
```rust
use snapshot_fonts::{SnapshotFillLevels, SnapshotCandles};

// Price sparkline
let chart = SnapshotFillLevels::from_prices(&prices).draw();

// Candlestick chart
let chart = SnapshotCandles::from_prices(&prices).draw();
```

# Unicode Ranges

## FillLevels
- Start: U+F09E5
- End: U+FFFFD
- Count: 63,001 (251 × 251)
- Plane: 15 (PUA-A)

## Candles
- Start: U+100000
- End: U+10CCD5
- Count: 52,438
- Plane: 16 (PUA-B)


<br>

<sup>
	This repository follows <a href="https://github.com/valeratrades/.github/tree/master/best_practices">my best practices</a> and <a href="https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/TIGER_STYLE.md">Tiger Style</a> (except "proper capitalization for acronyms": (VsrState, not VSRState) and formatting).
</sup>

#### License

<sup>
	Licensed under <a href="LICENSE">Blue Oak 1.0.0</a>
</sup>

<br>

<sub>
	Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be licensed as above, without any additional terms or conditions.
</sub>

