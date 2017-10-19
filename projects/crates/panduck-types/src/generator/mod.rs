#![doc = include_str!("readme.md")]

/// Text writer for Panduck target emitters.
#[derive(Debug)]
pub struct TextWriter<W> {
    writer: W,
    indent_level: u16,
    indent_text: &'static str,
}

impl<W> TextWriter<W> {
    /// Wraps a [`std::fmt::Write`] sink with Panduck indentation defaults.
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            indent_level: 0,
            indent_text: "    ",
        }
    }
}

impl<W: std::fmt::Write> TextWriter<W> {
    /// Increases the indentation level by one.
    pub fn indent(&mut self) {
        self.indent_level = self.indent_level.saturating_add(1);
    }

    /// Decreases the indentation level by one.
    pub fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }

    /// Writes one indented line terminated by a newline.
    pub fn write_line(&mut self, text: &str) -> std::fmt::Result {
        self.write_indent()?;
        writeln!(self.writer, "{}", text)
    }

    /// Writes raw text without indentation or a trailing newline.
    pub fn write(&mut self, text: &str) -> std::fmt::Result {
        write!(self.writer, "{}", text)
    }

    /// Writes the current indentation prefix.
    pub fn write_indent(&mut self) -> std::fmt::Result {
        for _ in 0..self.indent_level {
            write!(self.writer, "{}", self.indent_text)?;
        }
        Ok(())
    }

    /// Returns the active indentation depth.
    pub fn indent_level(&self) -> u16 {
        self.indent_level
    }

    /// Returns the wrapped writer and drops indentation state.
    pub fn finish(self) -> W {
        self.writer
    }
}
