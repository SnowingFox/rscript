//! Flag types and type-related definitions for the AST.
//!
//! Faithfully ports TypeScript's NodeFlags, ModifierFlags, TypeFlags, etc.

use std::fmt;

bitflags::bitflags! {
    /// Flags for AST nodes, matching TypeScript's NodeFlags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct NodeFlags: u32 {
        const NONE                          = 0;
        const LET                           = 1 << 0;
        const CONST                         = 1 << 1;
        const USING                         = 1 << 2;
        const AWAIT_USING                   = 1 << 3;
        const NESTED_NAMESPACE              = 1 << 4;
        const SYNTHESIZED                   = 1 << 5;
        const NAMESPACE                     = 1 << 6;
        const OPTIONAL_CHAIN                = 1 << 7;
        const EXPORT_CONTEXT                = 1 << 8;
        const CONTAINS_THIS                 = 1 << 9;
        const HAS_IMPLICIT_RETURN           = 1 << 10;
        const HAS_EXPLICIT_RETURN           = 1 << 11;
        const GLOBAL_AUGMENTATION           = 1 << 12;
        const HAS_ASYNC_FUNCTIONS           = 1 << 13;
        const DISALLOW_IN_CONTEXT           = 1 << 14;
        const YIELD_CONTEXT                 = 1 << 15;
        const DECORATOR_CONTEXT             = 1 << 16;
        const AWAIT_CONTEXT                 = 1 << 17;
        const DISALLOW_CONDITIONAL_TYPES_CONTEXT = 1 << 18;
        const THIS_NODE_HAS_ERROR           = 1 << 19;
        const JAVASCRIPT_FILE               = 1 << 20;
        const THIS_NODE_OR_ANY_SUB_NODES_HAS_ERROR = 1 << 21;
        const HAS_AGGREGATED_CHILD_DATA     = 1 << 22;
        const JSX                           = 1 << 23;

        const BLOCK_SCOPED = Self::LET.bits() | Self::CONST.bits() | Self::USING.bits() | Self::AWAIT_USING.bits();
        const CONSTANT = Self::CONST.bits() | Self::ENUM_MEMBER.bits();
        const ENUM_MEMBER                   = 1 << 24;

        const CONTEXT_FLAGS = Self::DISALLOW_IN_CONTEXT.bits()
            | Self::YIELD_CONTEXT.bits()
            | Self::DECORATOR_CONTEXT.bits()
            | Self::AWAIT_CONTEXT.bits()
            | Self::DISALLOW_CONDITIONAL_TYPES_CONTEXT.bits()
            | Self::JAVASCRIPT_FILE.bits();
    }
}

bitflags::bitflags! {
    /// Modifier flags for declarations, matching TypeScript's ModifierFlags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ModifierFlags: u32 {
        const NONE              = 0;
        const EXPORT            = 1 << 0;
        const AMBIENT           = 1 << 1;
        const PUBLIC            = 1 << 2;
        const PRIVATE           = 1 << 3;
        const PROTECTED         = 1 << 4;
        const STATIC            = 1 << 5;
        const READONLY          = 1 << 6;
        const ACCESSOR          = 1 << 7;
        const ABSTRACT          = 1 << 8;
        const ASYNC             = 1 << 9;
        const DEFAULT           = 1 << 10;
        const CONST             = 1 << 11;
        const DEPRECATED        = 1 << 12;
        const OVERRIDE          = 1 << 13;
        const IN                = 1 << 14;
        const OUT               = 1 << 15;
        const DECORATOR         = 1 << 16;

        const ACCESSIBILITY_MODIFIER = Self::PUBLIC.bits() | Self::PRIVATE.bits() | Self::PROTECTED.bits();
        const PARAMETER_PROPERTY_MODIFIER = Self::ACCESSIBILITY_MODIFIER.bits() | Self::READONLY.bits() | Self::OVERRIDE.bits();
        const NON_PUBLIC_ACCESSIBILITY_MODIFIER = Self::PRIVATE.bits() | Self::PROTECTED.bits();

        const TYPE_SCRIPT_MODIFIER = Self::AMBIENT.bits()
            | Self::PUBLIC.bits()
            | Self::PRIVATE.bits()
            | Self::PROTECTED.bits()
            | Self::READONLY.bits()
            | Self::ABSTRACT.bits()
            | Self::CONST.bits()
            | Self::OVERRIDE.bits()
            | Self::IN.bits()
            | Self::OUT.bits();

        const EXPORT_DEFAULT = Self::EXPORT.bits() | Self::DEFAULT.bits();
        const ALL = Self::EXPORT.bits()
            | Self::AMBIENT.bits()
            | Self::PUBLIC.bits()
            | Self::PRIVATE.bits()
            | Self::PROTECTED.bits()
            | Self::STATIC.bits()
            | Self::READONLY.bits()
            | Self::ACCESSOR.bits()
            | Self::ABSTRACT.bits()
            | Self::ASYNC.bits()
            | Self::DEFAULT.bits()
            | Self::CONST.bits()
            | Self::DEPRECATED.bits()
            | Self::OVERRIDE.bits()
            | Self::IN.bits()
            | Self::OUT.bits()
            | Self::DECORATOR.bits();
    }
}

bitflags::bitflags! {
    /// Type flags used by the type checker, matching TypeScript's TypeFlags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TypeFlags: u32 {
        const NONE              = 0;
        const ANY               = 1 << 0;
        const UNKNOWN           = 1 << 1;
        const STRING            = 1 << 2;
        const NUMBER            = 1 << 3;
        const BOOLEAN           = 1 << 4;
        const ENUM              = 1 << 5;
        const BIG_INT           = 1 << 6;
        const STRING_LITERAL    = 1 << 7;
        const NUMBER_LITERAL    = 1 << 8;
        const BOOLEAN_LITERAL   = 1 << 9;
        const ENUM_LITERAL      = 1 << 10;
        const BIG_INT_LITERAL   = 1 << 11;
        const ES_SYMBOL         = 1 << 12;
        const UNIQUE_ES_SYMBOL  = 1 << 13;
        const VOID              = 1 << 14;
        const UNDEFINED         = 1 << 15;
        const NULL              = 1 << 16;
        const NEVER             = 1 << 17;
        const TYPE_PARAMETER    = 1 << 18;
        const OBJECT            = 1 << 19;
        const UNION             = 1 << 20;
        const INTERSECTION      = 1 << 21;
        const INDEX             = 1 << 22;
        const INDEXED_ACCESS    = 1 << 23;
        const CONDITIONAL       = 1 << 24;
        const SUBSTITUTION      = 1 << 25;
        const NON_PRIMITIVE     = 1 << 26;
        const TEMPLATE_LITERAL  = 1 << 27;
        const STRING_MAPPING    = 1 << 28;

        const LITERAL = Self::STRING_LITERAL.bits()
            | Self::NUMBER_LITERAL.bits()
            | Self::BOOLEAN_LITERAL.bits()
            | Self::ENUM_LITERAL.bits()
            | Self::BIG_INT_LITERAL.bits();

        const UNIT = Self::LITERAL.bits()
            | Self::UNIQUE_ES_SYMBOL.bits()
            | Self::UNDEFINED.bits()
            | Self::NULL.bits();

        const STRING_OR_NUMBER_LITERAL = Self::STRING_LITERAL.bits() | Self::NUMBER_LITERAL.bits();

        const STRING_LIKE = Self::STRING.bits() | Self::STRING_LITERAL.bits() | Self::TEMPLATE_LITERAL.bits() | Self::STRING_MAPPING.bits();
        const NUMBER_LIKE = Self::NUMBER.bits() | Self::NUMBER_LITERAL.bits() | Self::ENUM.bits();
        const BIG_INT_LIKE = Self::BIG_INT.bits() | Self::BIG_INT_LITERAL.bits();
        const BOOLEAN_LIKE = Self::BOOLEAN.bits() | Self::BOOLEAN_LITERAL.bits();
        const ENUM_LIKE = Self::ENUM.bits() | Self::ENUM_LITERAL.bits();
        const ES_SYMBOL_LIKE = Self::ES_SYMBOL.bits() | Self::UNIQUE_ES_SYMBOL.bits();
        const VOID_LIKE = Self::VOID.bits() | Self::UNDEFINED.bits();
        const PRIMITIVE = Self::STRING.bits()
            | Self::NUMBER.bits()
            | Self::BIG_INT.bits()
            | Self::BOOLEAN.bits()
            | Self::ENUM.bits()
            | Self::ES_SYMBOL.bits()
            | Self::VOID.bits()
            | Self::UNDEFINED.bits()
            | Self::NULL.bits()
            | Self::LITERAL.bits()
            | Self::UNIQUE_ES_SYMBOL.bits();

        const UNION_OR_INTERSECTION = Self::UNION.bits() | Self::INTERSECTION.bits();

        const DEFINITELY_FALSY = Self::STRING_LITERAL.bits()
            | Self::NUMBER_LITERAL.bits()
            | Self::BIG_INT_LITERAL.bits()
            | Self::BOOLEAN_LITERAL.bits()
            | Self::VOID.bits()
            | Self::UNDEFINED.bits()
            | Self::NULL.bits();

        const POSSIBLY_FALSY = Self::DEFINITELY_FALSY.bits()
            | Self::STRING.bits()
            | Self::NUMBER.bits()
            | Self::BIG_INT.bits()
            | Self::BOOLEAN.bits();

        const NARROWABLE = Self::ANY.bits()
            | Self::UNKNOWN.bits()
            | Self::STRING.bits()
            | Self::NUMBER.bits()
            | Self::BIG_INT.bits()
            | Self::BOOLEAN.bits()
            | Self::ENUM.bits()
            | Self::ES_SYMBOL.bits()
            | Self::OBJECT.bits()
            | Self::UNION.bits()
            | Self::NEVER.bits()
            | Self::VOID.bits()
            | Self::UNDEFINED.bits()
            | Self::NULL.bits()
            | Self::LITERAL.bits()
            | Self::UNIQUE_ES_SYMBOL.bits()
            | Self::NON_PRIMITIVE.bits()
            | Self::TEMPLATE_LITERAL.bits()
            | Self::STRING_MAPPING.bits()
            | Self::TYPE_PARAMETER.bits()
            | Self::INDEXED_ACCESS.bits()
            | Self::CONDITIONAL.bits()
            | Self::SUBSTITUTION.bits()
            | Self::INTERSECTION.bits()
            | Self::INDEX.bits();
    }
}

bitflags::bitflags! {
    /// Symbol flags used by the binder, matching TypeScript's SymbolFlags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SymbolFlags: u32 {
        const NONE                          = 0;
        const FUNCTION_SCOPED_VARIABLE      = 1 << 0;
        const BLOCK_SCOPED_VARIABLE         = 1 << 1;
        const PROPERTY                      = 1 << 2;
        const ENUM_MEMBER                   = 1 << 3;
        const FUNCTION                      = 1 << 4;
        const CLASS                         = 1 << 5;
        const INTERFACE                     = 1 << 6;
        const CONST_ENUM                    = 1 << 7;
        const REGULAR_ENUM                  = 1 << 8;
        const VALUE_MODULE                  = 1 << 9;
        const NAMESPACE_MODULE              = 1 << 10;
        const TYPE_LITERAL                  = 1 << 11;
        const OBJECT_LITERAL                = 1 << 12;
        const METHOD                        = 1 << 13;
        const CONSTRUCTOR                   = 1 << 14;
        const GET_ACCESSOR                  = 1 << 15;
        const SET_ACCESSOR                  = 1 << 16;
        const SIGNATURE                     = 1 << 17;
        const TYPE_PARAMETER                = 1 << 18;
        const TYPE_ALIAS                    = 1 << 19;
        const EXPORT_VALUE                  = 1 << 20;
        const ALIAS                         = 1 << 21;
        const PROTOTYPE                     = 1 << 22;
        const EXPORT_STAR                   = 1 << 23;
        const OPTIONAL                      = 1 << 24;
        const TRANSIENT                     = 1 << 25;
        const ASSIGNMENT                    = 1 << 26;
        const MODULE_EXPORTS                = 1 << 27;
        /// Visibility and modifier flags on symbols (mirror of ModifierFlags for binder use).
        const PRIVATE                       = 1 << 28;
        const PROTECTED                     = 1 << 29;
        const STATIC                        = 1 << 30;

        const ENUM = Self::REGULAR_ENUM.bits() | Self::CONST_ENUM.bits();
        const VARIABLE = Self::FUNCTION_SCOPED_VARIABLE.bits() | Self::BLOCK_SCOPED_VARIABLE.bits();
        const VALUE = Self::VARIABLE.bits()
            | Self::PROPERTY.bits()
            | Self::ENUM_MEMBER.bits()
            | Self::OBJECT_LITERAL.bits()
            | Self::FUNCTION.bits()
            | Self::CLASS.bits()
            | Self::ENUM.bits()
            | Self::VALUE_MODULE.bits()
            | Self::METHOD.bits()
            | Self::GET_ACCESSOR.bits()
            | Self::SET_ACCESSOR.bits();
        const TYPE = Self::CLASS.bits()
            | Self::INTERFACE.bits()
            | Self::ENUM.bits()
            | Self::ENUM_MEMBER.bits()
            | Self::TYPE_LITERAL.bits()
            | Self::TYPE_PARAMETER.bits()
            | Self::TYPE_ALIAS.bits();
        const NAMESPACE = Self::VALUE_MODULE.bits()
            | Self::NAMESPACE_MODULE.bits()
            | Self::ENUM.bits();
        const MODULE = Self::VALUE_MODULE.bits() | Self::NAMESPACE_MODULE.bits();
        const ACCESSOR = Self::GET_ACCESSOR.bits() | Self::SET_ACCESSOR.bits();

        const CLASS_MEMBER = Self::METHOD.bits()
            | Self::ACCESSOR.bits()
            | Self::PROPERTY.bits();
    }
}

bitflags::bitflags! {
    /// Object type flags, matching TypeScript's ObjectFlags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ObjectFlags: u32 {
        const NONE               = 0;
        const CLASS              = 1 << 0;
        const INTERFACE          = 1 << 1;
        const REFERENCE          = 1 << 2;
        const TUPLE              = 1 << 3;
        const ANONYMOUS          = 1 << 4;
        const MAPPED             = 1 << 5;
        const INSTANTIATED       = 1 << 6;
        const OBJECT_LITERAL     = 1 << 7;
        const EVOLVING_ARRAY     = 1 << 8;
        const OBJECT_LITERAL_PATTERN_WITH_COMPUTED_PROPERTIES = 1 << 9;
        const REVERSE_MAPPED     = 1 << 10;
        const JS_LITERAL         = 1 << 11;
        const FRESH_LITERAL      = 1 << 12;
        const ARRAY_LITERAL      = 1 << 13;
        const CLASS_OR_INTERFACE = Self::CLASS.bits() | Self::INTERFACE.bits();
    }
}

// Token flags from the scanner.
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TokenFlags: u16 {
        const NONE                              = 0;
        const PRECEDING_LINE_BREAK              = 1 << 0;
        const PRECEDING_JSX_TEXT_LINE_BREAK      = 1 << 1;
        const UNTERMINATED                       = 1 << 2;
        const EXTENDED_UNICODE_ESCAPE            = 1 << 3;
        const SCIENTIFIC                         = 1 << 4;
        const OCTAL                              = 1 << 5;
        const HEX_SPECIFIER                      = 1 << 6;
        const BINARY_SPECIFIER                   = 1 << 7;
        const OCTAL_SPECIFIER                    = 1 << 8;
        const CONTAINS_SEPARATOR                 = 1 << 9;
        const UNICODE_ESCAPE                     = 1 << 10;
        const CONTAINS_INVALID_ESCAPE            = 1 << 11;
        const HAS_EXTENDED_UNICODE_ESCAPE        = 1 << 12;
        const IS_INVALID                         = 1 << 13;

        const NUMERIC_LITERAL_FLAGS = Self::SCIENTIFIC.bits()
            | Self::OCTAL.bits()
            | Self::HEX_SPECIFIER.bits()
            | Self::BINARY_SPECIFIER.bits()
            | Self::OCTAL_SPECIFIER.bits()
            | Self::CONTAINS_SEPARATOR.bits();
    }
}

// Flow node flags for control flow analysis.
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FlowFlags: u16 {
        const UNREACHABLE      = 1 << 0;
        const START            = 1 << 1;
        const BRANCH_LABEL     = 1 << 2;
        const LOOP_LABEL       = 1 << 3;
        const ASSIGNMENT       = 1 << 4;
        const TRUE_CONDITION   = 1 << 5;
        const FALSE_CONDITION  = 1 << 6;
        const SWITCH_CLAUSE    = 1 << 7;
        const ARRAY_MUTATION   = 1 << 8;
        const CALL             = 1 << 9;
        const REDUCE_LABEL     = 1 << 10;
        const REFERENCED       = 1 << 11;
        const SHARED           = 1 << 12;

        const LABEL = Self::BRANCH_LABEL.bits() | Self::LOOP_LABEL.bits();
        const CONDITION = Self::TRUE_CONDITION.bits() | Self::FALSE_CONDITION.bits();
    }
}

/// The type ID is a lightweight handle to a type stored in the type arena.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct TypeId(pub u32);

impl TypeId {
    pub const INVALID: TypeId = TypeId(u32::MAX);

    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TypeId({})", self.0)
    }
}

/// The symbol ID is a lightweight handle to a symbol in the symbol table.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct SymbolId(pub u32);

impl SymbolId {
    pub const INVALID: SymbolId = SymbolId(u32::MAX);

    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Node ID for referencing AST nodes by index.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct NodeId(pub u32);

impl NodeId {
    pub const INVALID: NodeId = NodeId(u32::MAX);

    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Source file ID for referencing source files.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct SourceFileId(pub u32);
