// SPDX-License-Identifier: GPL-3.0-or-later
// FoamCraft report layout. All content comes from /data.json (text only, never markup);
// images are /logo.png, /chart.svg and /view-<key>.png.
#let d = json("/data.json")
#let accent = rgb("#1d4f73")
#let light-colour(level) = if level == "green" { rgb("#3a9d5d") } else if level == "yellow" { rgb("#c98f0f") } else { rgb("#c8423a") }
#let label(key) = d.labels.at(key, default: "")

#set document(title: d.title, author: d.cover.at(3, default: ("", "")).at(1))
#set text(font: "Libertinus Serif", size: 10.5pt, lang: d.lang)
#set par(justify: true, leading: 0.62em)
#set page(
  paper: "a4",
  margin: (x: 2.1cm, top: 2.2cm, bottom: 2.6cm),
  footer: context {
    set text(7pt, fill: luma(110))
    set par(justify: false)
    grid(columns: (1fr, auto), gutter: 8pt, d.trademark.at(-1), counter(page).display())
  },
)
#set heading(numbering: "1.")
#show heading: it => {
  set text(fill: accent)
  v(0.6em)
  it
  v(0.2em)
}

#let pairs(rows) = table(
  columns: (auto, 1fr),
  stroke: (x, y) => (bottom: 0.4pt + luma(220)),
  inset: (x: 4pt, y: 5pt),
  ..rows.map(r => (strong(r.at(0)), r.at(1))).flatten(),
)

#let badge(level, text-content) = box(
  fill: light-colour(level),
  inset: (x: 9pt, y: 4pt),
  radius: 8pt,
  text(fill: white, weight: "bold", text-content),
)

// Cover.
#page(footer: none)[
  #v(2.5cm)
  #if d.logo { image("/logo.png", height: 2cm) }
  #v(1cm)
  #text(26pt, weight: "bold", fill: accent, d.title)
  #v(2mm)
  #text(14pt, fill: luma(80), d.title_kind)
  #v(1.2cm)
  #pairs(d.cover)
  #v(8mm)
  #badge(d.reliability.level, d.reliability.label)
  #v(1fr)
  #set text(8pt, fill: luma(100))
  #for t in d.trademark [#t #parbreak()]
]

#outline(title: label("contents"), depth: 1)
#pagebreak()

#for s in d.sections {
  heading(level: 1, label("section_" + s))
  if s == "summary" {
    for p in d.summary [#p #parbreak()]
  } else if s == "setup" {
    pairs(d.setup)
  } else if s == "mesh" {
    pairs(d.mesh)
  } else if s == "convergence" {
    pairs(d.convergence.rows)
    if d.chart {
      figure(image("/chart.svg", width: 100%), caption: d.convergence.chart_title)
    }
    if d.convergence.note != none { text(fill: luma(90), d.convergence.note) }
  } else if s == "figures" {
    table(
      columns: (auto, auto, auto, 1fr),
      stroke: (x, y) => (bottom: 0.4pt + luma(220)),
      inset: (x: 4pt, y: 5pt),
      align: (left, right, left, left),
      table.header([], strong(label("figures_value")), strong(label("figures_unit")), strong(label("figures_meaning"))),
      ..d.figures.map(f => (strong(f.label), f.value, f.unit, text(9pt, f.text))).flatten(),
    )
  } else if s == "views" {
    for v in d.views {
      figure(image("/view-" + v.key + ".png", width: 100%), caption: [#v.title – #v.caption])
    }
    if d.views_hint != none { text(fill: luma(90), d.views_hint) }
  } else if s == "reliability" {
    badge(d.reliability.level, d.reliability.label)
    v(4pt)
    for c in d.reliability.checks {
      grid(columns: (10pt, 1fr), gutter: 6pt, circle(radius: 3.5pt, fill: light-colour(c.at(0))), c.at(1))
      v(2pt)
    }
  } else if s == "comparison" and d.comparison != none {
    let c = d.comparison
    table(
      columns: (1fr, auto, auto, auto, auto),
      stroke: (x, y) => (bottom: 0.4pt + luma(220)),
      inset: (x: 4pt, y: 5pt),
      align: (left, right, right, right, right),
      table.header(..c.header.map(h => strong(h))),
      ..c.rows.flatten(),
    )
  } else if s == "limits" {
    list(..d.limits)
  } else if s == "versions" {
    pairs(d.versions)
  }
}

#heading(level: 1, numbering: none, label("section_trademark"))
#for t in d.trademark [#t #parbreak()]
