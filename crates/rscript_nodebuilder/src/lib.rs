//! rscript_nodebuilder: Synthetic AST node construction.
//!
//! Creates AST nodes for type display in error messages and
//! declaration emit (.d.ts generation).

pub struct NodeBuilder;

impl NodeBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Generate a type declaration string from a name and type.
    /// 
    /// # Example
    /// ```
    /// use rscript_nodebuilder::NodeBuilder;
    /// let builder = NodeBuilder::new();
    /// let decl = builder.build_declaration("x", "number");
    /// assert_eq!(decl, "declare const x: number;");
    /// ```
    pub fn build_declaration(&self, name: &str, type_string: &str) -> String {
        format!("declare const {}: {};", name, type_string)
    }

    /// Generate a function declaration.
    /// 
    /// # Example
    /// ```
    /// use rscript_nodebuilder::NodeBuilder;
    /// let builder = NodeBuilder::new();
    /// let decl = builder.build_function_declaration(
    ///     "add",
    ///     &[("a".to_string(), "number".to_string()), ("b".to_string(), "number".to_string())],
    ///     "number"
    /// );
    /// assert_eq!(decl, "declare function add(a: number, b: number): number;");
    /// ```
    pub fn build_function_declaration(
        &self,
        name: &str,
        params: &[(String, String)],
        return_type: &str,
    ) -> String {
        let params_str = params
            .iter()
            .map(|(param_name, param_type)| format!("{}: {}", param_name, param_type))
            .collect::<Vec<_>>()
            .join(", ");
        format!("declare function {}({}): {};", name, params_str, return_type)
    }

    /// Generate an interface declaration.
    /// 
    /// # Example
    /// ```
    /// use rscript_nodebuilder::NodeBuilder;
    /// let builder = NodeBuilder::new();
    /// let decl = builder.build_interface_declaration(
    ///     "Point",
    ///     &[("x".to_string(), "number".to_string()), ("y".to_string(), "number".to_string())]
    /// );
    /// assert_eq!(decl, "interface Point {\n    x: number;\n    y: number;\n}");
    /// ```
    pub fn build_interface_declaration(
        &self,
        name: &str,
        members: &[(String, String)],
    ) -> String {
        if members.is_empty() {
            format!("interface {} {{\n}}", name)
        } else {
            let members_str = members
                .iter()
                .map(|(member_name, member_type)| format!("    {}: {};", member_name, member_type))
                .collect::<Vec<_>>()
                .join("\n");
            format!("interface {} {{\n{}\n}}", name, members_str)
        }
    }

    /// Generate a type alias declaration.
    /// 
    /// # Example
    /// ```
    /// use rscript_nodebuilder::NodeBuilder;
    /// let builder = NodeBuilder::new();
    /// let decl = builder.build_type_alias("ID", "string | number");
    /// assert_eq!(decl, "type ID = string | number;");
    /// ```
    pub fn build_type_alias(&self, name: &str, type_body: &str) -> String {
        format!("type {} = {};", name, type_body)
    }
}

impl Default for NodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_declaration() {
        let builder = NodeBuilder::new();
        assert_eq!(
            builder.build_declaration("x", "number"),
            "declare const x: number;"
        );
        assert_eq!(
            builder.build_declaration("name", "string"),
            "declare const name: string;"
        );
    }

    #[test]
    fn test_build_function_declaration() {
        let builder = NodeBuilder::new();
        assert_eq!(
            builder.build_function_declaration("add", &[], "number"),
            "declare function add(): number;"
        );
        assert_eq!(
            builder.build_function_declaration(
                "add",
                &[("a".to_string(), "number".to_string()), ("b".to_string(), "number".to_string())],
                "number"
            ),
            "declare function add(a: number, b: number): number;"
        );
        assert_eq!(
            builder.build_function_declaration(
                "greet",
                &[("name".to_string(), "string".to_string())],
                "void"
            ),
            "declare function greet(name: string): void;"
        );
    }

    #[test]
    fn test_build_interface_declaration() {
        let builder = NodeBuilder::new();
        assert_eq!(
            builder.build_interface_declaration("Point", &[]),
            "interface Point {\n}"
        );
        assert_eq!(
            builder.build_interface_declaration(
                "Point",
                &[("x".to_string(), "number".to_string()), ("y".to_string(), "number".to_string())]
            ),
            "interface Point {\n    x: number;\n    y: number;\n}"
        );
        assert_eq!(
            builder.build_interface_declaration(
                "User",
                &[
                    ("id".to_string(), "number".to_string()),
                    ("name".to_string(), "string".to_string()),
                    ("email".to_string(), "string | null".to_string())
                ]
            ),
            "interface User {\n    id: number;\n    name: string;\n    email: string | null;\n}"
        );
    }

    #[test]
    fn test_build_type_alias() {
        let builder = NodeBuilder::new();
        assert_eq!(
            builder.build_type_alias("ID", "string | number"),
            "type ID = string | number;"
        );
        assert_eq!(
            builder.build_type_alias("Callback", "() => void"),
            "type Callback = () => void;"
        );
        assert_eq!(
            builder.build_type_alias("Nullable", "T | null"),
            "type Nullable = T | null;"
        );
    }
}
