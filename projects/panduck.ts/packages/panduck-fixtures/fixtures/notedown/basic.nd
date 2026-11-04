# Oak Markdown Test Suite

This is a comprehensive test file for the Oak Markdown lexer, covering various Markdown syntax elements.

## Headings

### Different Heading Levels

# Heading 1
## Heading 2
### Heading 3
#### Heading 4
##### Heading 5
###### Heading 6

## Text Formatting

### Emphasis and Strong

This text contains *italic* and **bold** formatting.

You can also use _underscores_ for italic and __double underscores__ for bold.

### Combined Formatting

This text is ***bold and italic*** and can also be written as **_bold and italic_**.

### Strikethrough

This text has been ~~struck through~~.

## Code

### Inline Code

Use `inline code` for short code snippets within text.

### Code Blocks

```rust
fn main() {
    println!("Hello, world!");
}
```

```javascript
function greet(name) {
    return `Hello, ${name}!`;
}
```

```python
def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n-1) + fibonacci(n-2)
```

### Code Block with Language

```bash
#!/bin/bash
echo "This is a bash script"
ls -la
```

## Lists

### Unordered Lists

- First item
- Second item
- Third item

* Alternative bullet style
* Another item
* Final item

+ Plus style bullet
+ Another item
+ Last item

### Ordered Lists

1. First step
2. Second step
3. Third step

42. Number can start at any value
43. Second item
44. Third item

### Nested Lists

1. First main item
   - Sub-item A
   - Sub-item B
2. Second main item
   - Sub-item C
   * Alternative sub-bullet
   + Another style

### Task Lists

- [ ] Unchecked task
- [x] Completed task
- [X] Also completed task
- [ ] Another unchecked task

## Links and Images

### Basic Links

[Oak Parser Framework](https://github.com/oak/oak)

[Link with title](https://example.com "This is a title")

### Reference Links

[Reference link][ref1]

[Another reference][ref2]

[ref1]: https://example.com "Reference 1"
[ref2]: https://another.com 'Reference 2'

### Images

![Oak Logo](https://oak.dev/logo.png)

![Image with alt text](image.jpg "Image title")

## Blockquotes

> This is a simple blockquote.

> This is a longer blockquote that spans
> multiple lines. Each line starts with
> the greater-than symbol.

### Nested Blockquotes

> First level of quoting
>> Second level of quoting
>>> Third level of quoting

### Blockquote with Other Elements

> ### Heading in Blockquote
> 
> This is **bold** text in a blockquote.
> 
> - List item in blockquote
> - Another item
> 
> ```
> Code block in blockquote
> ```

## Tables

### Basic Table

| Header 1 | Header 2 | Header 3 |
|----------|----------|----------|
| Cell 1   | Cell 2   | Cell 3   |
| Cell 4   | Cell 5   | Cell 6   |

### Aligned Table

| Left Aligned | Center Aligned | Right Aligned |
|:-------------|:--------------:|--------------:|
| Left         | Center         | Right         |
| Text         | Text           | Text          |

### Complex Table

| Feature | Description | Status |
|---------|-------------|--------|
| Lexer   | Tokenizes input | ✅ |
| Parser  | Builds AST | ✅ |
| Linter  | Finds issues | 🚧 |

## Horizontal Rules

---

***

___

- - -

* * *

_ _ _

## Special Elements

### Escaping Characters

Use \*escaped asterisks\* and \`escaped backticks\`.

### HTML Entities

This text contains &copy; copyright symbol and &trade; trademark symbol.

### Line Breaks

This line ends with two spaces  
Which creates a line break.

This is a new paragraph.

### Emojis (if supported)

🎉 🚀 💻 📚 🔧

## Advanced Features

### Definition Lists

Term 1
: Definition 1

Term 2
: Definition 2a
: Definition 2b

### Footnotes

Here's a sentence with a footnote[^1].

[^1]: This is the footnote.

### Admonitions (if supported)

> **Note:** This is a note admonition.
> 
> **Warning:** This is a warning admonition.

### Math (if supported)

When $a \ne 0$, there are two solutions to $(ax^2 + bx + c = 0)$.

$$
\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}
$$

## Mixed Content

This paragraph contains **bold**, *italic*, `code`, and [links](https://example.com).

> A blockquote with **formatting** and [links](https://example.com).

- A list item with **bold** text
- Another item with `inline code`
- [ ] A task with a [link](https://example.com)

```markdown
# Code block with Markdown
This is a **code block** that contains Markdown syntax.
```

---

*This document serves as a comprehensive test suite for the Oak Markdown lexer, covering all major Markdown syntax elements and edge cases.*