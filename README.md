
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
![Build time](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/snapshot_fonts-build.json)
![Test time](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/snapshot_fonts-test.json)
<br>
[![errors](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/snapshot_fonts-errors.json)](https://github.com/valeratrades/snapshot_fonts/actions/workflows/errors.yml?query=branch%3Amain)
[![warnings](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/snapshot_fonts-warnings.json)](https://github.com/valeratrades/snapshot_fonts/actions/workflows/warnings.yml?query=branch%3Amain)

Monospace fonts for terminal-based data visualization. Encode chart data directly in Unicode text.

- **FillLevels**: 251×251 fill level combinations for sparklines/histograms
- **Candles**: 52k candlestick glyphs for financial charts

### Standard vs Custom Font Resolution
normally when trying to render series of meaningful values in snapshot tests (or terminal), you are limited to constructing them from just 8 bar chars (`' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'`), which is severely limiting.  
I couldn't take it anymore, hence this project. Here is the side-by-side comparison of the default method (top) vs 2x255 per chart resolution with our custom font

![Standard 9-level block characters](./docs/.readme_assets/levels.png)

### Candlestick Charts
And here are the candlesticks (max precision I could fit into the available slots):

![Candlestick chart format](./docs/.readme_assets/candles.png)
<!-- markdownlint-disable -->
<details>
<summary>
<h2>Installation</h2>
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

## Other
### Unicode Ranges
#### FillLevels
- Start: U+F09E5
- End: U+FFFFD
- Count: 63,001 (251 × 251)
- Plane: 15 (PUA-A)

#### Candles
- Start: U+100000
- End: U+10CCD5
- Count: 52,438
- Plane: 16 (PUA-B)


<br>

<sup>Links for devs:</sup>
- <sup><a href="./docs/ARCHITECTURE.md">architecture</a></sup>
- <sup><a href="https://github.com/EV-Main/.github/blob/main/code/STYLE.md">style</a></sup>
- <sup><a href="https://github.com/EV-Main/.github/tree/main/code/conventions">conventions</a></sup>
- <sup><a href="https://github.com/EV-Main/.github/blob/main/code/conventions/git">contributing</a></sup>

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

