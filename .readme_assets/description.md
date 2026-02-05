Monospace fonts for terminal-based data visualization. Encode chart data directly in Unicode text.

- **FillLevels**: 251×251 fill level combinations for sparklines/histograms
- **Candles**: 52k candlestick glyphs for financial charts

## Standard vs Custom Font Resolution

Standard terminal charts are limited to 9 block characters (`' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'`), resulting in coarse vertical resolution:

![Standard 9-level block characters](fallback_vs_custom.png)

With our custom font, we achieve much finer granularity - notice the smoother transitions and more detailed representation of the data.

## Candlestick Charts

Our implementation also supports candlestick charts for financial data visualization:

![Candlestick chart format](candles.png)
