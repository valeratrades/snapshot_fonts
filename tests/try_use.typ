//NB: don't forget to compile with `--font-path .`
#set page(width: auto, height: auto, margin: 1em)

// Two-bar glyph: char_code = left * 251 + right (with surrogate skip)
// left and right are 0-250
#let bars(left, right, size: 24pt) = {
  let code = left * 251 + right
  if code >= 0xD800 { code += 2048 }
  text(font: "FillLevels", fallback: false, size: size, str.from-unicode(code))
}

= Fill Level Font Test (Two Bars Per Char)

== Sample pairs:
#bars(250, 250) #bars(250, 125) #bars(125, 250) #bars(200, 50) #bars(50, 200)

== 4-step; towards each other
#for i in range(0, 251, step: 4) {
  bars(i, 250 - i)
}
