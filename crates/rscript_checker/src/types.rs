//! Type system representation.
//!
//! Types are stored in a TypeTable (type arena) and referenced by TypeId.
//! This avoids lifetime issues with recursive type structures.

use indexmap::IndexMap;
use rscript_ast::types::{ObjectFlags, TypeFlags, TypeId, SymbolId};

/// A type in the TypeScript type system.
#[derive(Debug, Clone)]
pub struct Type {
    /// Unique identifier.
    pub id: TypeId,
    /// Type flags describing what kind of type this is.
    pub flags: TypeFlags,
    /// The symbol associated with this type (if any).
    pub symbol: Option<SymbolId>,
    /// The specific kind of type.
    pub kind: TypeKind,
}

/// The specific data for each type kind.
#[derive(Debug, Clone)]
pub enum TypeKind {
    /// Intrinsic types: any, unknown, string, number, boolean, void, undefined, null, never
    Intrinsic {
        name: &'static str,
    },
    /// String literal type
    StringLiteral {
        value: String,
        regular: bool,
    },
    /// Number literal type
    NumberLiteral {
        value: f64,
    },
    /// BigInt literal type
    BigIntLiteral {
        value: String,
    },
    /// Boolean literal type (true/false)
    BooleanLiteral {
        value: bool,
    },
    /// Object type (class, interface, object literal, etc.)
    ObjectType {
        object_flags: ObjectFlags,
        members: IndexMap<String, TypeId>,
        call_signatures: Vec<Signature>,
        construct_signatures: Vec<Signature>,
        index_infos: Vec<IndexInfo>,
    },
    /// Union type (A | B | C)
    Union {
        types: Vec<TypeId>,
    },
    /// Intersection type (A & B & C)
    Intersection {
        types: Vec<TypeId>,
    },
    /// Type parameter (T)
    TypeParameter {
        constraint: Option<TypeId>,
        default: Option<TypeId>,
    },
    /// Indexed access type (T[K])
    IndexedAccess {
        object_type: TypeId,
        index_type: TypeId,
    },
    /// Conditional type (T extends U ? X : Y)
    Conditional {
        check_type: TypeId,
        extends_type: TypeId,
        true_type: TypeId,
        false_type: TypeId,
    },
    /// Mapped type ({ [K in T]: U })
    Mapped {
        type_parameter: TypeId,
        constraint_type: TypeId,
        template_type: Option<TypeId>,
    },
    /// Template literal type (`hello ${T}`)
    TemplateLiteral {
        texts: Vec<String>,
        types: Vec<TypeId>,
    },
    /// Substitution type (internal for conditional type distribution)
    Substitution {
        base_type: TypeId,
        constraint: TypeId,
    },
    /// Tuple type
    Tuple {
        element_types: Vec<TypeId>,
        element_flags: Vec<ElementFlags>,
    },
    /// Type reference (generic instantiation)
    TypeReference {
        target: TypeId,
        type_arguments: Vec<TypeId>,
    },
}

/// A function/method signature.
#[derive(Debug, Clone)]
pub struct Signature {
    pub type_parameters: Vec<TypeId>,
    pub parameters: Vec<SignatureParameter>,
    pub return_type: TypeId,
    pub min_argument_count: u32,
    pub has_rest_parameter: bool,
}

/// A parameter in a signature.
#[derive(Debug, Clone)]
pub struct SignatureParameter {
    pub name: String,
    pub type_id: TypeId,
    pub optional: bool,
}

/// An index signature (string or number indexer).
#[derive(Debug, Clone)]
pub struct IndexInfo {
    pub key_type: TypeId,
    pub type_id: TypeId,
    pub is_readonly: bool,
}

/// Element flags for tuple types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementFlags {
    Required,
    Optional,
    Rest,
    Variadic,
}

/// The type table stores all types and provides access by TypeId.
#[derive(Debug)]
pub struct TypeTable {
    types: Vec<Type>,
    // Well-known types
    pub any_type: TypeId,
    pub unknown_type: TypeId,
    pub string_type: TypeId,
    pub number_type: TypeId,
    pub boolean_type: TypeId,
    pub void_type: TypeId,
    pub undefined_type: TypeId,
    pub null_type: TypeId,
    pub never_type: TypeId,
    pub bigint_type: TypeId,
    pub symbol_type: TypeId,
    pub object_type: TypeId,
    pub true_type: TypeId,
    pub false_type: TypeId,
}

impl TypeTable {
    pub fn new() -> Self {
        let mut table = Self {
            types: Vec::with_capacity(1024),
            any_type: TypeId(0),
            unknown_type: TypeId(1),
            string_type: TypeId(2),
            number_type: TypeId(3),
            boolean_type: TypeId(4),
            void_type: TypeId(5),
            undefined_type: TypeId(6),
            null_type: TypeId(7),
            never_type: TypeId(8),
            bigint_type: TypeId(9),
            symbol_type: TypeId(10),
            object_type: TypeId(11),
            true_type: TypeId(12),
            false_type: TypeId(13),
        };

        // Create intrinsic types
        table.create_intrinsic(TypeFlags::ANY, "any");
        table.create_intrinsic(TypeFlags::UNKNOWN, "unknown");
        table.create_intrinsic(TypeFlags::STRING, "string");
        table.create_intrinsic(TypeFlags::NUMBER, "number");
        table.create_intrinsic(TypeFlags::BOOLEAN, "boolean");
        table.create_intrinsic(TypeFlags::VOID, "void");
        table.create_intrinsic(TypeFlags::UNDEFINED, "undefined");
        table.create_intrinsic(TypeFlags::NULL, "null");
        table.create_intrinsic(TypeFlags::NEVER, "never");
        table.create_intrinsic(TypeFlags::BIG_INT, "bigint");
        table.create_intrinsic(TypeFlags::ES_SYMBOL, "symbol");
        table.create_intrinsic(TypeFlags::NON_PRIMITIVE, "object");
        // true/false literal types
        table.add_type(TypeFlags::BOOLEAN_LITERAL, TypeKind::BooleanLiteral { value: true });
        table.add_type(TypeFlags::BOOLEAN_LITERAL, TypeKind::BooleanLiteral { value: false });

        table
    }

    fn create_intrinsic(&mut self, flags: TypeFlags, name: &'static str) -> TypeId {
        self.add_type(flags, TypeKind::Intrinsic { name })
    }

    /// Add a new type to the table and return its ID.
    pub fn add_type(&mut self, flags: TypeFlags, kind: TypeKind) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(Type {
            id,
            flags,
            symbol: None,
            kind,
        });
        id
    }

    /// Get a type by its ID.
    pub fn get(&self, id: TypeId) -> &Type {
        &self.types[id.index()]
    }

    /// Get a mutable reference to a type by its ID.
    pub fn get_mut(&mut self, id: TypeId) -> &mut Type {
        &mut self.types[id.index()]
    }

    /// Get the total number of types.
    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }
}

impl Default for TypeTable {
    fn default() -> Self {
        Self::new()
    }
}
