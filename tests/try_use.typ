//NB: don't forget to compile with `--font-path .`
#set page(width: auto, height: auto, margin: 1em)

// Helper to render a bar at a given fill level (0-255)
#let bar(level, size: 24pt) = {
  text(font: "FillLevels", fallback: false, size: size, str.from-unicode(level))
}

= Fill Level Font Test

== Bar chart (values: 255, 200, 128, 64, 32):
#bar(255)#bar(200)#bar(128)#bar(64)#bar(32)

== Gradient (every 16th level from 0 to 255):
#for i in range(0, 256, step: 16) {
  bar(i)
}#bar(255)

== gradient every 4th:
#for i in range(0, 256, step: 4) {
  bar(i)
}
