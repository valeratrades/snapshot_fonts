Monospace fonts for terminal-based data visualization. Encode chart data directly in Unicode text.

- **FillLevels**: 251×251 fill level combinations for sparklines/histograms
- **Candles**: 52k candlestick glyphs for financial charts

## Standard vs Custom Font Resolution
normally when trying to render series of meaningful values in snapshot tests (or terminal), you are limited to constructing them from just 8 bar chars (`' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'`), which is severely limiting.  
I couldn't take it anymore, hence this project. Here is the side-by-side comparison of the default method (top) vs 2x255 per chart resolution with our custom font

![Standard 9-level block characters](./levels.png)

## Candlestick Charts
And here are the candlesticks (max precision I could fit into the available slots):

![Candlestick chart format](./candles.png)
