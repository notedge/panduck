# Document Family Report

## Overview

This report covers **bold**, *italic*, ~~strike~~ and `inline code` plus a [link](https://example.com) and inline math $E=mc^2$.

### Scope

The fixture drives real exports instead of a two-paragraph stub. It exercises headings, lists, a table, an image, a footnote, a math block, a diagram fence and a chart fence in one document.

## Lists

- first bullet
  - nested bullet
- second bullet with *emphasis*

1. one
2. two
3. three

## Table

| Name | Value | Note |
| --- | --- | --- |
| alpha | 1 | ok |
| beta | 2 | deferred |

## Code

```rust
fn main() { println!("hello"); }
```

## Math

$$
\int_0^1 x^2 dx = \frac{1}{3}
$$

## Diagram

```mermaid
graph LR
  A --> B
  B --> C
```

## Chart

```vega-lite
{"$schema":"https://vega.github.io/schema/vega-lite/v5.json","data":{"values":[{"x":"A","y":3},{"x":"B","y":7}]},"mark":"bar","encoding":{"x":{"field":"x","type":"nominal"},"y":{"field":"y","type":"quantitative"}}}
```

## Image

![pixel](pixel.png)

## Footnote

A statement with a footnote.[^1]

[^1]: The footnote body.
