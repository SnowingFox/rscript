//! rscript_sourcemap: Source map generation.
//!
//! Generates V3 source maps for mapping output JS/DTS back to
//! original TypeScript source.

/// Base64 VLQ alphabet used in source maps.
const VLQ_BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encode a signed integer as a Base64 VLQ string.
///
/// VLQ (Variable Length Quantity) encoding uses a base-64 encoding where:
/// - Each 6-bit Base64 character encodes 5 bits of data plus a continuation bit
/// - The continuation bit (bit 5) indicates if there are more digits
/// - The sign bit (bit 0) is in the first character (0 = positive, 1 = negative)
/// - The value bits are in bits 1-4 of the first character, and bits 0-4 of subsequent characters
pub fn encode_vlq(value: i64) -> String {
    let mut result = String::new();
    let mut num = if value < 0 {
        ((-value) << 1) | 1
    } else {
        value << 1
    };

    loop {
        let mut digit = (num & 0x1F) as u8; // Get 5 bits
        num >>= 5;
        if num > 0 {
            digit |= 0x20; // Set continuation bit (bit 5)
        }
        result.push(VLQ_BASE64[digit as usize] as char);
        if num == 0 {
            break;
        }
    }

    result
}

/// A source map builder that accumulates mappings.
#[allow(dead_code)]
pub struct SourceMapBuilder {
    mappings: Vec<Mapping>,
    sources: Vec<String>,
    names: Vec<String>,
}

/// A single mapping entry.
#[derive(Debug, Clone)]
pub struct Mapping {
    pub generated_line: u32,
    pub generated_column: u32,
    pub source_index: Option<u32>,
    pub original_line: Option<u32>,
    pub original_column: Option<u32>,
    pub name_index: Option<u32>,
}

impl SourceMapBuilder {
    pub fn new() -> Self {
        Self {
            mappings: Vec::new(),
            sources: Vec::new(),
            names: Vec::new(),
        }
    }

    /// Add a source file and return its index.
    pub fn add_source(&mut self, file: &str) -> u32 {
        let idx = self.sources.len() as u32;
        self.sources.push(file.to_string());
        idx
    }

    /// Add a mapping entry.
    pub fn add_mapping(
        &mut self,
        generated_line: u32,
        generated_column: u32,
        source_index: u32,
        original_line: u32,
        original_column: u32,
    ) {
        self.mappings.push(Mapping {
            generated_line,
            generated_column,
            source_index: Some(source_index),
            original_line: Some(original_line),
            original_column: Some(original_column),
            name_index: None,
        });
    }

    /// Encode the source map as a JSON string.
    pub fn to_json(&self) -> String {
        // Sort mappings by generated line and column
        let mut sorted_mappings = self.mappings.clone();
        sorted_mappings.sort_by(|a, b| {
            a.generated_line
                .cmp(&b.generated_line)
                .then_with(|| a.generated_column.cmp(&b.generated_column))
        });

        // Build mappings string with VLQ encoding
        let mut mappings_str = String::new();
        let mut prev_generated_line = 0u32;
        let mut prev_generated_column = 0i64;
        let mut prev_source_index = 0i64;
        let mut prev_original_line = 0i64;
        let mut prev_original_column = 0i64;

        for (i, mapping) in sorted_mappings.iter().enumerate() {
            if i > 0 && mapping.generated_line != prev_generated_line {
                mappings_str.push(';');
                prev_generated_column = 0;
                // Note: source/original values continue from previous segment
            } else if i > 0 {
                mappings_str.push(',');
            }
            
            prev_generated_line = mapping.generated_line;

            // Generated column (relative)
            let col_diff = mapping.generated_column as i64 - prev_generated_column;
            mappings_str.push_str(&encode_vlq(col_diff));
            prev_generated_column = mapping.generated_column as i64;

            // If we have source information, encode it
            if let (Some(source_idx), Some(orig_line), Some(orig_col)) = (
                mapping.source_index,
                mapping.original_line,
                mapping.original_column,
            ) {
                // Source index (relative)
                let source_diff = source_idx as i64 - prev_source_index;
                mappings_str.push_str(&encode_vlq(source_diff));
                prev_source_index = source_idx as i64;

                // Original line (relative)
                let line_diff = orig_line as i64 - prev_original_line;
                mappings_str.push_str(&encode_vlq(line_diff));
                prev_original_line = orig_line as i64;

                // Original column (relative)
                let col_diff = orig_col as i64 - prev_original_column;
                mappings_str.push_str(&encode_vlq(col_diff));
                prev_original_column = orig_col as i64;

                // Name index (if present)
                if let Some(name_idx) = mapping.name_index {
                    mappings_str.push_str(&encode_vlq(name_idx as i64));
                }
            }
        }

        // Build JSON
        let mut json = format!("{{\"version\":3,\"sources\":[");
        for (i, source) in self.sources.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push('"');
            json.push_str(&escape_json_string(source));
            json.push('"');
        }
        json.push_str("],\"names\":[");
        for (i, name) in self.names.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push('"');
            json.push_str(&escape_json_string(name));
            json.push('"');
        }
        json.push_str("],\"mappings\":\"");
        json.push_str(&mappings_str);
        json.push_str("\"}");

        json
    }
}

/// Escape special characters in a JSON string.
fn escape_json_string(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '"' => vec!['\\', '"'],
            '\\' => vec!['\\', '\\'],
            '\n' => vec!['\\', 'n'],
            '\r' => vec!['\\', 'r'],
            '\t' => vec!['\\', 't'],
            '\u{0008}' => vec!['\\', 'b'],
            '\u{000C}' => vec!['\\', 'f'],
            c => vec![c],
        })
        .collect()
}

impl Default for SourceMapBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_vlq_zero() {
        assert_eq!(encode_vlq(0), "A");
    }

    #[test]
    fn test_encode_vlq_positive() {
        assert_eq!(encode_vlq(1), "C");
        assert_eq!(encode_vlq(2), "E");
        assert_eq!(encode_vlq(15), "e");
        // 16 << 1 = 32, which requires 2 characters: 'g' (0 with continuation) + 'B' (1)
        assert_eq!(encode_vlq(16), "gB");
    }

    #[test]
    fn test_encode_vlq_negative() {
        assert_eq!(encode_vlq(-1), "D");
        assert_eq!(encode_vlq(-2), "F");
        assert_eq!(encode_vlq(-15), "f");
        // -16: (-16) << 1 | 1 = 33, which requires 2 characters: 'h' (1 with continuation) + 'B' (1)
        assert_eq!(encode_vlq(-16), "hB");
    }

    #[test]
    fn test_encode_vlq_large_values() {
        // Test values that require multiple Base64 characters
        let result = encode_vlq(1000);
        assert!(!result.is_empty());
        // 1000 << 1 = 2000, which requires multiple 5-bit chunks
        assert!(result.len() > 1);
    }

    #[test]
    fn test_source_map_builder_basic() {
        let mut builder = SourceMapBuilder::new();
        let source_idx = builder.add_source("test.ts");
        assert_eq!(source_idx, 0);

        builder.add_mapping(0, 0, source_idx, 0, 0);
        builder.add_mapping(0, 10, source_idx, 0, 5);

        let json = builder.to_json();
        assert!(json.contains("\"version\":3"));
        assert!(json.contains("\"sources\":[\"test.ts\"]"));
        assert!(json.contains("\"mappings\":"));
    }

    #[test]
    fn test_source_map_builder_multiple_sources() {
        let mut builder = SourceMapBuilder::new();
        let idx1 = builder.add_source("file1.ts");
        let idx2 = builder.add_source("file2.ts");
        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);

        builder.add_mapping(0, 0, idx1, 0, 0);
        builder.add_mapping(0, 10, idx2, 0, 5);

        let json = builder.to_json();
        assert!(json.contains("\"file1.ts\""));
        assert!(json.contains("\"file2.ts\""));
    }

    #[test]
    fn test_source_map_builder_multiple_lines() {
        let mut builder = SourceMapBuilder::new();
        let source_idx = builder.add_source("test.ts");

        builder.add_mapping(0, 0, source_idx, 0, 0);
        builder.add_mapping(1, 0, source_idx, 1, 0);
        builder.add_mapping(2, 5, source_idx, 2, 10);

        let json = builder.to_json();
        // Mappings should contain semicolons to separate lines
        assert!(json.contains(";"));
    }

    #[test]
    fn test_source_map_builder_relative_encoding() {
        let mut builder = SourceMapBuilder::new();
        let source_idx = builder.add_source("test.ts");

        // Add mappings on the same line to test relative encoding
        builder.add_mapping(0, 0, source_idx, 0, 0);
        builder.add_mapping(0, 10, source_idx, 0, 5);
        builder.add_mapping(0, 20, source_idx, 0, 15);

        let json = builder.to_json();
        // Should have commas separating segments on the same line
        assert!(json.contains(","));
    }

    #[test]
    fn test_json_escaping() {
        let mut builder = SourceMapBuilder::new();
        builder.add_source("file with \"quotes\".ts");
        builder.add_source("file\\with\\backslashes.ts");

        let json = builder.to_json();
        // Quotes and backslashes should be escaped
        assert!(json.contains("\\\""));
        assert!(json.contains("\\\\"));
    }

    #[test]
    fn test_encode_vlq_edge_cases() {
        // Test boundary values
        assert_eq!(encode_vlq(0), "A");
        assert_eq!(encode_vlq(-1), "D");
        assert_eq!(encode_vlq(1), "C");
        
        // Test values that fit in one character (0-15)
        assert_eq!(encode_vlq(15), "e");
        assert_eq!(encode_vlq(-15), "f");
        
        // Test values requiring multiple characters
        assert_eq!(encode_vlq(31), "+B"); // 31 << 1 = 62, needs 2 chars
        assert_eq!(encode_vlq(-31), "/B");
        
        // Test large values
        let large_positive = encode_vlq(1000);
        assert!(!large_positive.is_empty());
        assert!(large_positive.len() > 1);
        
        let large_negative = encode_vlq(-1000);
        assert!(!large_negative.is_empty());
        assert!(large_negative.len() > 1);
    }

    #[test]
    fn test_source_map_empty() {
        let builder = SourceMapBuilder::new();
        let json = builder.to_json();
        assert!(json.contains("\"version\":3"));
        assert!(json.contains("\"sources\":[]"));
        assert!(json.contains("\"names\":[]"));
        assert!(json.contains("\"mappings\":\"\""));
    }

    #[test]
    fn test_source_map_complex_scenario() {
        let mut builder = SourceMapBuilder::new();
        let source1 = builder.add_source("file1.ts");
        let source2 = builder.add_source("file2.ts");
        
        // Add mappings across multiple lines and sources
        builder.add_mapping(0, 0, source1, 0, 0);
        builder.add_mapping(0, 10, source1, 0, 5);
        builder.add_mapping(1, 0, source2, 0, 0);
        builder.add_mapping(1, 15, source2, 0, 10);
        builder.add_mapping(2, 5, source1, 1, 3);
        
        let json = builder.to_json();
        
        // Verify structure
        assert!(json.contains("\"version\":3"));
        assert!(json.contains("\"file1.ts\""));
        assert!(json.contains("\"file2.ts\""));
        assert!(json.contains("\"mappings\":"));
        
        // Verify mappings contain separators
        assert!(json.contains(";") || json.contains(","));
    }

    #[test]
    fn test_source_map_json_structure() {
        let mut builder = SourceMapBuilder::new();
        builder.add_source("test.ts");
        builder.add_mapping(0, 0, 0, 0, 0);
        
        let json = builder.to_json();
        
        // Verify it's valid JSON structure
        assert!(json.starts_with("{"));
        assert!(json.ends_with("}"));
        assert!(json.contains("\"version\":3"));
        assert!(json.contains("\"sources\":"));
        assert!(json.contains("\"names\":"));
        assert!(json.contains("\"mappings\":"));
    }

    #[test]
    fn test_vlq_roundtrip_consistency() {
        // Test that encoding produces consistent results
        const VLQ_BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let values = vec![0, 1, -1, 15, -15, 16, -16, 31, -31, 100, -100, 1000, -1000];
        for &val in &values {
            let encoded = encode_vlq(val);
            // Each encoded value should be non-empty and contain only Base64 characters
            assert!(!encoded.is_empty());
            for ch in encoded.chars() {
                assert!(VLQ_BASE64.contains(&(ch as u8)));
            }
        }
    }
}
