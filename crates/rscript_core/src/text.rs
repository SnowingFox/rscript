//! Text span and range types for source location tracking.
//!
//! These types are used throughout the compiler to track where AST nodes,
//! tokens, and diagnostics originate in the source code.

use std::fmt;
use std::ops::Range;

/// A position in source text, measured as a byte offset from the start.
pub type TextPos = u32;

/// A span in source text, defined by a start position and a length.
/// This matches TypeScript's `TextSpan` interface.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct TextSpan {
    /// The byte offset where this span starts.
    pub start: TextPos,
    /// The length of this span in bytes.
    pub length: TextPos,
}

impl TextSpan {
    /// Create a new text span.
    #[inline]
    pub fn new(start: TextPos, length: TextPos) -> Self {
        Self { start, length }
    }

    /// Create a span from start and end positions.
    #[inline]
    pub fn from_bounds(start: TextPos, end: TextPos) -> Self {
        debug_assert!(end >= start);
        Self {
            start,
            length: end - start,
        }
    }

    /// Create an empty span at a position.
    #[inline]
    pub fn empty(pos: TextPos) -> Self {
        Self {
            start: pos,
            length: 0,
        }
    }

    /// The end position of this span (exclusive).
    #[inline]
    pub fn end(&self) -> TextPos {
        self.start + self.length
    }

    /// Whether this span is empty (zero-length).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Whether this span contains the given position.
    #[inline]
    pub fn contains(&self, pos: TextPos) -> bool {
        pos >= self.start && pos < self.end()
    }

    /// Whether this span contains or touches the given position.
    #[inline]
    pub fn contains_inclusive(&self, pos: TextPos) -> bool {
        pos >= self.start && pos <= self.end()
    }

    /// Whether this span overlaps with another span.
    #[inline]
    pub fn overlaps(&self, other: &TextSpan) -> bool {
        self.start < other.end() && other.start < self.end()
    }

    /// Convert to a byte range.
    #[inline]
    pub fn to_range(&self) -> Range<usize> {
        self.start as usize..self.end() as usize
    }

    /// Return a new span covering both this span and the other.
    pub fn union(&self, other: &TextSpan) -> TextSpan {
        let start = self.start.min(other.start);
        let end = self.end().max(other.end());
        TextSpan::from_bounds(start, end)
    }
}

impl fmt::Debug for TextSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end())
    }
}

impl fmt::Display for TextSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {})", self.start, self.end())
    }
}

/// A text range with start and end positions.
/// This matches TypeScript's `TextRange` interface.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct TextRange {
    /// The byte offset where this range starts (inclusive).
    pub pos: TextPos,
    /// The byte offset where this range ends (exclusive).
    pub end: TextPos,
}

impl TextRange {
    /// Create a new text range.
    #[inline]
    pub fn new(pos: TextPos, end: TextPos) -> Self {
        Self { pos, end }
    }

    /// Create an empty range at a position.
    #[inline]
    pub fn empty(pos: TextPos) -> Self {
        Self { pos, end: pos }
    }

    /// The length of this range in bytes.
    #[inline]
    pub fn len(&self) -> TextPos {
        self.end - self.pos
    }

    /// Whether this range is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pos == self.end
    }

    /// Convert to a TextSpan.
    #[inline]
    pub fn to_span(&self) -> TextSpan {
        TextSpan::from_bounds(self.pos, self.end)
    }

    /// Convert to a byte range.
    #[inline]
    pub fn to_range(&self) -> Range<usize> {
        self.pos as usize..self.end as usize
    }

    /// Whether this range contains a position.
    #[inline]
    pub fn contains(&self, pos: TextPos) -> bool {
        pos >= self.pos && pos < self.end
    }
}

impl fmt::Debug for TextRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.pos, self.end)
    }
}

impl From<TextRange> for TextSpan {
    fn from(range: TextRange) -> Self {
        range.to_span()
    }
}

impl From<TextSpan> for TextRange {
    fn from(span: TextSpan) -> Self {
        TextRange::new(span.start, span.end())
    }
}

/// Line and column information derived from source text.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct LineAndColumn {
    /// 0-based line number.
    pub line: u32,
    /// 0-based column (in UTF-16 code units, matching TypeScript).
    pub character: u32,
}

impl LineAndColumn {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// A map from byte offsets to line numbers, built from source text.
/// This is used to convert byte offsets to line/column positions for diagnostics.
#[derive(Debug, Clone)]
pub struct LineMap {
    /// Byte offsets of the start of each line.
    line_starts: Vec<TextPos>,
}

impl LineMap {
    /// Build a line map from source text.
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0u32];
        for (i, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push((i + 1) as u32);
            }
        }
        Self { line_starts }
    }

    /// Get the line number (0-based) for a byte offset.
    pub fn line_of(&self, pos: TextPos) -> u32 {
        match self.line_starts.binary_search(&pos) {
            Ok(line) => line as u32,
            Err(line) => (line - 1) as u32,
        }
    }

    /// Get the line and column for a byte offset.
    pub fn line_and_column_of(&self, pos: TextPos) -> LineAndColumn {
        let line = self.line_of(pos);
        let line_start = self.line_starts[line as usize];
        LineAndColumn {
            line,
            character: pos - line_start,
        }
    }

    /// Get the byte offset of the start of a line.
    pub fn line_start(&self, line: u32) -> TextPos {
        self.line_starts[line as usize]
    }

    /// Get the total number of lines.
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Get all line starts.
    pub fn line_starts(&self) -> &[TextPos] {
        &self.line_starts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_span() {
        let span = TextSpan::new(5, 10);
        assert_eq!(span.start, 5);
        assert_eq!(span.length, 10);
        assert_eq!(span.end(), 15);
        assert!(span.contains(5));
        assert!(span.contains(14));
        assert!(!span.contains(15));
    }

    #[test]
    fn test_text_span_from_bounds() {
        let span = TextSpan::from_bounds(5, 15);
        assert_eq!(span.start, 5);
        assert_eq!(span.length, 10);
    }

    #[test]
    fn test_line_map() {
        let text = "line1\nline2\nline3";
        let map = LineMap::new(text);
        assert_eq!(map.line_count(), 3);
        assert_eq!(map.line_of(0), 0);
        assert_eq!(map.line_of(5), 0); // newline char
        assert_eq!(map.line_of(6), 1); // start of line2
        assert_eq!(map.line_of(12), 2);

        let lc = map.line_and_column_of(8);
        assert_eq!(lc.line, 1);
        assert_eq!(lc.character, 2);
    }
}
