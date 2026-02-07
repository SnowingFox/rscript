//! rscript_transformers: AST transformations.
//!
//! Transforms the AST for various purposes:
//! - ES target downleveling (e.g., async/await -> generators)
//! - JSX transformation
//! - Decorator transformation
//! - TypeScript stripping (remove type annotations for JS emit)

use regex::Regex;

/// A transformer that modifies the AST.
pub trait Transformer {
    /// Transform a source file AST.
    fn transform<'a>(&self, node: &rscript_ast::node::SourceFile<'a>) -> rscript_ast::node::SourceFile<'a>;
}

/// Strip TypeScript-specific syntax for JavaScript emit.
pub struct TypeScriptStripper;

impl TypeScriptStripper {
    /// Strip type annotations from TypeScript source to produce JavaScript.
    /// 
    /// This function removes:
    /// - Type annotations after `:` in parameter lists and variable declarations
    /// - `interface` and `type` declarations entirely
    /// - `as T` type assertions (keeping just the expression)
    /// - `<T>` type parameters from function declarations
    /// 
    /// # Example
    /// ```
    /// use rscript_transformers::TypeScriptStripper;
    /// let stripper = TypeScriptStripper;
    /// let result = stripper.strip_types("function add(a: number, b: number): number { return a + b; }");
    /// assert_eq!(result, "function add(a, b) { return a + b; }");
    /// ```
    pub fn strip_types(&self, source: &str) -> String {
        let mut result = source.to_string();
        
        // Remove interface declarations (multiline)
        let interface_re = Regex::new(r"(?m)^\s*export\s+interface\s+\w+\s*\{[^}]*\}\s*$").unwrap();
        result = interface_re.replace_all(&result, "").to_string();
        let interface_re = Regex::new(r"(?m)^\s*interface\s+\w+\s*\{[^}]*\}\s*$").unwrap();
        result = interface_re.replace_all(&result, "").to_string();
        
        // Remove type alias declarations
        let type_re = Regex::new(r"(?m)^\s*export\s+type\s+\w+\s*=\s*[^;]+;\s*$").unwrap();
        result = type_re.replace_all(&result, "").to_string();
        let type_re = Regex::new(r"(?m)^\s*type\s+\w+\s*=\s*[^;]+;\s*$").unwrap();
        result = type_re.replace_all(&result, "").to_string();
        
        // Remove type parameters from function declarations: function name<T>(...)
        let generic_fn_re = Regex::new(r"(\w+)\s*<[^>]+>\s*\(").unwrap();
        result = generic_fn_re.replace_all(&result, "$1(").to_string();
        
        // Remove type annotations from variable declarations FIRST (before parameter types)
        // let x: type = ... -> let x = ...
        let var_type_re = Regex::new(r"(let|const|var)\s+(\w+)\s*:\s*[^=;]+\s*=").unwrap();
        result = var_type_re.replace_all(&result, "$1 $2 =").to_string();
        
        // Remove return type annotations: ): returnType { or ): returnType =>
        // Need to preserve the space before { or =>
        let return_type_re = Regex::new(r"\)\s*:\s*[^{=>\s]+(\s*\{|\s*=>)").unwrap();
        result = return_type_re.replace_all(&result, ")$1").to_string();
        
        // Remove type annotations from function parameters: (param: type) -> (param)
        // This handles both function declarations and arrow functions
        let param_type_re = Regex::new(r"(\w+)\s*:\s*[^,)]+").unwrap();
        result = param_type_re.replace_all(&result, "$1").to_string();
        
        // Remove 'as T' type assertions: expr as Type -> expr
        // Match expressions that can include parentheses, but stop at semicolon or end of line
        // This handles cases like: distance(...) as number, x as Type, etc.
        let as_type_re = Regex::new(r"([^;]+?)\s+as\s+\w+").unwrap();
        result = as_type_re.replace_all(&result, "$1").to_string();
        
        // Clean up multiple consecutive newlines
        let newline_re = Regex::new(r"\n\s*\n\s*\n+").unwrap();
        result = newline_re.replace_all(&result, "\n\n").to_string();
        
        result.trim().to_string()
    }
}

/// Transform JSX to function calls.
pub struct JsxTransformer;

/// Transform decorators.
pub struct DecoratorTransformer;

/// Downlevel ES features to older targets.
pub struct EsDownlevelTransformer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_function_parameter_types() {
        let stripper = TypeScriptStripper;
        let input = "function add(a: number, b: number): number { return a + b; }";
        let output = stripper.strip_types(input);
        assert_eq!(output, "function add(a, b) { return a + b; }");
    }

    #[test]
    fn test_strip_arrow_function_types() {
        let stripper = TypeScriptStripper;
        let input = "const add = (a: number, b: number): number => a + b;";
        let output = stripper.strip_types(input);
        assert_eq!(output, "const add = (a, b) => a + b;");
    }

    #[test]
    fn test_strip_variable_declaration_types() {
        let stripper = TypeScriptStripper;
        let input = "let x: number = 5; const name: string = 'test';";
        let output = stripper.strip_types(input);
        assert_eq!(output, "let x = 5; const name = 'test';");
    }

    #[test]
    fn test_strip_interface_declarations() {
        let stripper = TypeScriptStripper;
        let input = "interface Point { x: number; y: number; }";
        let output = stripper.strip_types(input);
        assert_eq!(output.trim(), "");
    }

    #[test]
    fn test_strip_export_interface_declarations() {
        let stripper = TypeScriptStripper;
        let input = "export interface User { id: number; name: string; }";
        let output = stripper.strip_types(input);
        assert_eq!(output.trim(), "");
    }

    #[test]
    fn test_strip_type_alias_declarations() {
        let stripper = TypeScriptStripper;
        let input = "type ID = string | number;";
        let output = stripper.strip_types(input);
        assert_eq!(output.trim(), "");
    }

    #[test]
    fn test_strip_export_type_alias_declarations() {
        let stripper = TypeScriptStripper;
        let input = "export type Callback = () => void;";
        let output = stripper.strip_types(input);
        assert_eq!(output.trim(), "");
    }

    #[test]
    fn test_strip_type_assertions() {
        let stripper = TypeScriptStripper;
        let input = "const value = x as number;";
        let output = stripper.strip_types(input);
        assert_eq!(output, "const value = x;");
    }

    #[test]
    fn test_strip_generic_function_types() {
        let stripper = TypeScriptStripper;
        let input = "function identity<T>(x: T): T { return x; }";
        let output = stripper.strip_types(input);
        assert_eq!(output, "function identity(x) { return x; }");
    }

    #[test]
    fn test_strip_complex_example() {
        let stripper = TypeScriptStripper;
        let input = r#"
interface Point {
    x: number;
    y: number;
}

function distance(p1: Point, p2: Point): number {
    return Math.sqrt((p2.x - p1.x) ** 2 + (p2.y - p1.y) ** 2);
}

const p1: Point = { x: 0, y: 0 };
const result = distance(p1, { x: 3, y: 4 }) as number;
"#;
        let output = stripper.strip_types(input);
        // Should remove interface, parameter types, return types, variable types, and type assertion
        assert!(!output.contains("interface Point"));
        assert!(!output.contains(": Point"));
        assert!(!output.contains(": number"));
        assert!(!output.contains("as number"));
        assert!(output.contains("function distance"));
        assert!(output.contains("const p1"));
    }
}
