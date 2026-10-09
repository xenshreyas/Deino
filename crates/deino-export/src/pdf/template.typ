// Deino's meeting PDF (EXP-01, ADR 0022).
//
// The meeting arrives as data.json (pdf/model.rs). Every string from it is
// placed as text, never evaluated as markup. Colours are the light-theme
// design tokens (docs/references/ui.md §2.2); type is Inter at the app's
// scale, with 600 as the only bold weight.

#let data = json("data.json")

#let ink = rgb("#1F1F1C")
#let muted = rgb("#65655F")
#let subtle = rgb("#8C8C85")
#let hairline = rgb("#E6E6E1")
#let raised = rgb("#F5F5F2")
#let accent = rgb("#2F6E4E")
#let speakers = (
  rgb("#2F5FB3"), rgb("#7048B8"), rgb("#1B6F6F"),
  rgb("#A8551A"), rgb("#B03A75"), rgb("#6B6A1F"),
)

#set document(title: data.title, author: "Deino")
#set page(
  paper: "us-letter",
  margin: (x: 0.95in, top: 0.85in, bottom: 0.9in),
  footer: context {
    set text(size: 7.5pt, fill: subtle)
    data.title
    h(1fr)
    counter(page).display("1 / 1", both: true)
  },
)
#set text(font: "Inter", size: 9.5pt, fill: ink, lang: "en", features: ("cv11",))
#set par(leading: 0.68em, spacing: 1.05em)
#set list(indent: 0.15em, body-indent: 0.6em, spacing: 0.7em, marker: text(fill: subtle, sym.bullet))
#set enum(indent: 0.15em, body-indent: 0.5em, spacing: 0.7em, numbering: n => text(fill: muted, str(n) + "."))
#set strong(delta: 200)
#show link: set text(fill: accent)
#show raw: it => box(fill: raised, inset: (x: 3pt, y: 1.5pt), radius: 3pt, text(size: 0.92em, it.text))

// Section headings, then note headings stepping down to muted labels.
#show heading: it => {
  let size = (13pt, 11pt, 10pt, 9.5pt, 9.5pt, 9.5pt).at(it.level - 1)
  let colour = if it.level >= 3 { muted } else { ink }
  block(above: if it.level == 1 { 24pt } else { 15pt }, below: if it.level == 1 { 9pt } else { 7pt }, sticky: true,
    text(size: size, weight: 600, fill: colour, tracking: if it.level == 1 { -0.01em } else { 0em }, it.body))
}

#let checkbox(done) = box(
  width: 8.5pt, height: 8.5pt, radius: 2pt, baseline: 1pt,
  stroke: if done { none } else { 0.8pt + subtle },
  fill: if done { accent } else { none },
  if done {
    curve(
      stroke: (paint: white, thickness: 1.1pt, cap: "round", join: "round"),
      curve.move((2pt, 4.4pt)), curve.line((3.7pt, 6.1pt)), curve.line((6.7pt, 2.6pt)),
    )
  },
)

#let task(done, body, details) = grid(
  columns: (12pt, 1fr), column-gutter: 4pt, row-gutter: 0pt,
  checkbox(done),
  {
    if done { text(fill: muted, body) } else { body }
    if details != "" { text(fill: muted, size: 8.5pt, [ #h(3pt)· #details]) }
  },
)

#let safe(href) = (
  href.starts-with("https://") or href.starts-with("http://") or href.starts-with("mailto:")
)

#let inlines(xs) = {
  for x in xs {
    if x.kind == "text" { x.text }
    else if x.kind == "strong" { strong(inlines(x.content)) }
    else if x.kind == "emphasis" { emph(inlines(x.content)) }
    else if x.kind == "strike" { strike(inlines(x.content)) }
    else if x.kind == "code" { raw(x.text) }
    else if x.kind == "link" {
      if safe(x.href) { link(x.href, inlines(x.content)) } else { inlines(x.content) }
    }
    else if x.kind == "break" { linebreak() }
  }
}

#let item-body(blocks, render) = {
  for b in blocks {
    if b.kind == "paragraph" { inlines(b.content) } else { render(b) }
  }
}

#let render-block(b) = {
  if b.kind == "heading" { heading(level: b.level, outlined: false, inlines(b.content)) }
  else if b.kind == "paragraph" { par(inlines(b.content)) }
  else if b.kind == "list" {
    if b.items.any(i => i.checked != none) {
      stack(spacing: 0.75em, ..b.items.map(i => task(i.checked == true, item-body(i.blocks, render-block), "")))
    } else if b.ordered {
      enum(..b.items.map(i => item-body(i.blocks, render-block)))
    } else {
      list(..b.items.map(i => item-body(i.blocks, render-block)))
    }
  }
  else if b.kind == "quote" {
    block(inset: (left: 10pt), stroke: (left: 1.5pt + hairline),
      text(fill: muted, for q in b.blocks { render-block(q) }))
  }
  else if b.kind == "code" {
    block(fill: raised, inset: 8pt, radius: 6pt, width: 100%, text(size: 8.5pt, b.text))
  }
  else if b.kind == "rule" { line(length: 100%, stroke: 0.6pt + hairline) }
}

#let section(s) = {
  heading(level: 1, s.heading)
  if s.kind == "names" {
    text(fill: ink, s.names.join(", "))
  } else if s.kind == "notes" {
    for b in s.blocks { render-block(b) }
  } else if s.kind == "tasks" {
    stack(spacing: 0.8em, ..s.items.map(i => task(i.done, i.title, i.details)))
  } else if s.kind == "transcript" {
    grid(
      columns: (2.6em, 1fr), column-gutter: 10pt,
      ..s.lines.map(l => (
        grid.cell(inset: (top: if l.continued { 10pt } else { 16pt }),
          text(size: 8pt, fill: subtle, number-width: "tabular", l.time)),
        grid.cell(inset: (top: if l.continued { 10pt } else { 16pt }),
          if l.continued { l.text } else {
            [#text(weight: 600, fill: speakers.at(l.colour), l.speaker) \ #l.text]
          }),
      )).flatten()
    )
  }
}

// ── The document ──
#text(size: 21pt, weight: 600, tracking: -0.02em, data.title)
#v(7pt, weak: true)
#text(size: 9pt, fill: muted, data.meta)
#if data.tags.len() > 0 {
  v(6pt, weak: true)
  for tag in data.tags {
    box(inset: (x: 6pt, y: 2.5pt), radius: 99pt, stroke: 0.6pt + hairline,
      text(size: 7.5pt, fill: muted, tag))
    h(4pt)
  }
}
#v(12pt, weak: true)
#line(length: 100%, stroke: 0.6pt + hairline)

#for s in data.sections { section(s) }
