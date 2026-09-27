#![allow(clippy::ptr_arg)]

//! Rust struct builder implementation macro
//!
//! ## Motivation
//! A derive macros to support a builder pattern for Rust:
//! - Everything except `Option<>` fields and explicitly defined `default` attribute in structs are required, so you
//!   don't need any additional attributes to indicate it, and the presence of required params
//!   is checked at the compile time (not at the runtime).
//! - To create new struct instances there is `::new` and an auxiliary init struct definition
//!   with only required fields (to compensate the Rust's named params inability).
//!
//! ## Usage:
//!
//! ```
//! // Import it
//! use rsb_derive::Builder;
//!
//! // And use it on your structs
//! #[derive(Clone,Builder)]
//! struct MyStructure {
//!     pub req_field1: String,
//!     pub req_field2: i32,
//!     pub opt_field1: Option<String>,
//!     pub opt_field2: Option<i32>
//! }
//!
//! let s1 : MyStructure =
//!     MyStructure::from(
//!         MyStructureInit {
//!              req_field1 : "hey".into(),
//!              req_field2 : 0
//!         }
//!     )
//!     .with_opt_field1("hey".into())
//!     .with_opt_field2(10);
//! ```
//!
//! The macros generates the following functions and instances for your structures:
//! - `with/without_<field_name>` : immutable setters for fields
//! - `<field_name>/reset_<field_name>` : mutable setters for fields
//! - `new` : factory method with required fields as arguments
//! - `From<>` instance from an an auxiliary init struct definition with only required fields.
//!   The init structure generated as `<YourStructureName>Init`. So, you can use `from(...)` or `into()`
//!   functions from it.
//!
//! ## Defaults
//!
//! ```
//! use rsb_derive::Builder;
//!
//! #[derive(Debug, Clone, PartialEq, Builder)]
//! struct StructWithDefault {
//!     pub req_field1: String,
//!     #[default="10"]
//!     pub req_field2: i32, // default here make this field behave like optional
//!
//!     pub opt_field1: Option<String>,
//!     #[default="Some(11)"]
//!     pub opt_field2: Option<i32> // default works also on optional fields
//! }
//! ```
//!
//! Details and source code: [https://github.com/abdolence/rust-struct-builder]: https://github.com/abdolence/rust-struct-builder
//!

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::*;
use syn::ext::IdentExt;
use syn::*;

#[proc_macro_derive(Builder, attributes(default))]
pub fn struct_builder_macro(input: TokenStream) -> TokenStream {
    let item: syn::Item = syn::parse(input).expect("failed to parse input");
    let span = Span::call_site();
    match item {
        Item::Struct(ref struct_item) => match struct_item.fields {
            Fields::Named(ref named_fields) => {
                let struct_name = &struct_item.ident;
                let struct_generic_params: Vec<&TypeParam> = struct_item
                    .generics
                    .params
                    .iter()
                    .filter_map(|ga| match ga {
                        GenericParam::Type(ref ty) => Some(ty),
                        _ => None,
                    })
                    .collect();

                let struct_generic_params_idents: Vec<&Ident> =
                    struct_generic_params.iter().map(|gp| &gp.ident).collect();

                let struct_lifetime_params: Vec<&LifetimeParam> = struct_item
                    .generics
                    .params
                    .iter()
                    .filter_map(|ga| match ga {
                        GenericParam::Lifetime(ref lt) => Some(lt),
                        _ => None,
                    })
                    .collect();

                let struct_generic_where_decl: proc_macro2::TokenStream = struct_item
                    .generics
                    .where_clause
                    .as_ref()
                    .map_or(quote! {}, |wh| quote! { #wh });

                let struct_fields = parse_fields(named_fields);

                let generated_factory_method = generate_factory_method(struct_name, &struct_fields);
                let generated_fields_methods = generate_fields_functions(&struct_fields);

                let generated_aux_init_struct = generate_init_struct(
                    struct_name,
                    &struct_fields,
                    &struct_generic_params,
                    &struct_generic_params_idents,
                    &struct_lifetime_params,
                    struct_item.generics.where_clause.as_ref(),
                );

                let struct_decl: proc_macro2::TokenStream = if struct_generic_params.is_empty()
                    && struct_lifetime_params.is_empty()
                {
                    quote! {
                        impl #struct_name
                    }
                } else {
                    quote! {
                        impl <#(#struct_lifetime_params),* #(#struct_generic_params),* > #struct_name <#(#struct_lifetime_params),*  #(#struct_generic_params_idents),* > #struct_generic_where_decl
                    }
                };

                let output = quote! {
                    #[allow(dead_code)]
                    #[allow(clippy::needless_update)]
                    #struct_decl {
                        #generated_factory_method
                        #(#generated_fields_methods)*
                    }

                    #generated_aux_init_struct
                };

                output.into()
            }
            _ => Error::new(span, "Builder works only on the structs with named fields")
                .to_compile_error()
                .into(),
        },
        _ => Error::new(span, "Builder derive works only on structs")
            .to_compile_error()
            .into(),
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone)]
enum ParsedType {
    StringType,
    ScalarType,
    OptionalType(Box<ParsedFieldType>),
}

impl ParsedType {
    fn is_option(&self) -> bool {
        matches!(self, ParsedType::OptionalType(_))
    }
}

#[derive(Clone)]
struct ParsedFieldType {
    field_type: Type,
    parsed_type: Option<ParsedType>,
    lifetime: Option<Lifetime>,
}

#[derive(Clone)]
struct ParsedField {
    ident: Ident,
    parsed_field_type: ParsedFieldType,
    default_tokens: Option<proc_macro2::TokenStream>,
    visibility: Visibility,
    docs: Vec<Attribute>,
}

impl ParsedField {
    fn is_option(&self) -> bool {
        self.parsed_field_type
            .parsed_type
            .as_ref()
            .filter(|t| t.is_option())
            .is_some()
    }

    fn is_required_field(&self) -> bool {
        !self.is_option() && self.default_tokens.is_none()
    }
}

#[inline]
fn parse_field_type(field_type: &Type) -> ParsedFieldType {
    match field_type {
        Type::Path(ref path) => {
            let full_type_path: &String = &path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<String>>()
                .join("::");

            let parsed_type = match full_type_path.as_str() {
                "String" | "std::string::String" => Some(ParsedType::StringType),
                "Option" | "std::option::Option" => {
                    let type_params = &path.path.segments.last().unwrap().arguments;
                    match type_params {
                        PathArguments::AngleBracketed(ref params) => {
                            params.args.first().and_then(|ga| match ga {
                                GenericArgument::Type(ref ty) => {
                                    Some(ParsedType::OptionalType(Box::from(parse_field_type(ty))))
                                }
                                _ => None,
                            })
                        }
                        _ => None,
                    }
                }
                "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
                | "u128" | "usize" => Some(ParsedType::ScalarType),
                _ => None,
            };

            ParsedFieldType {
                field_type: field_type.clone(),
                parsed_type,
                lifetime: None,
            }
        }
        Type::Reference(ref type_ref) => ParsedFieldType {
            lifetime: type_ref.lifetime.clone(),
            field_type: field_type.clone(),
            parsed_type: None,
        },
        _ => ParsedFieldType {
            field_type: field_type.clone(),
            parsed_type: None,
            lifetime: None,
        },
    }
}

fn parse_fields(fields: &FieldsNamed) -> Vec<ParsedField> {
    fields.named.iter().map(parse_field).collect()
}

fn parse_field(field: &Field) -> ParsedField {
    ParsedField {
        ident: field.ident.as_ref().unwrap().clone(),
        parsed_field_type: parse_field_type(&field.ty),
        default_tokens: parse_field_default_attr(field),
        visibility: field.vis.clone(),
        docs: parse_field_docs(field),
    }
}

/// The field's doc comments, i.e. its outer `#[doc = ...]` attributes. List
/// forms such as `#[doc(hidden)]` are not documentation text and stay on the
/// field alone.
fn parse_field_docs(field: &Field) -> Vec<Attribute> {
    field
        .attrs
        .iter()
        .filter(|a| {
            matches!(a.style, AttrStyle::Outer)
                && a.path().is_ident("doc")
                && matches!(a.meta, Meta::NameValue(_))
        })
        .cloned()
        .collect()
}

/// Doc attributes for a generated item: `summary` as the first paragraph,
/// followed by the field's own doc comments as a separate paragraph.
fn doc_block(summary: &str, field_docs: &[Attribute]) -> proc_macro2::TokenStream {
    // `///` expands to a doc string with a leading space; matching it keeps
    // rustdoc's common-indent stripping uniform across the summary and the
    // propagated lines.
    let summary = format!(" {summary}");
    if field_docs.is_empty() {
        quote! { #[doc = #summary] }
    } else {
        quote! {
            #[doc = #summary]
            #[doc = ""]
            #(#field_docs)*
        }
    }
}

fn generate_fields_functions(fields: &[ParsedField]) -> Vec<proc_macro2::TokenStream> {
    fields.iter().map(generate_field_functions).collect()
}

fn generate_field_functions(field: &ParsedField) -> proc_macro2::TokenStream {
    let field_name = &field.ident;
    let set_field_name = format_ident!("{}", field_name);
    let reset_field_name = format_ident!("reset_{}", field_name);
    let with_field_name = format_ident!("with_{}", field_name);
    let without_field_name = format_ident!("without_{}", field_name);
    let opt_field_name = format_ident!("opt_{}", field_name);
    let mut_opt_field_name = format_ident!("mopt_{}", field_name);

    let field_type = &field.parsed_field_type.field_type;
    let field_visibility = &field.visibility;

    let shown_name = field_name.unraw();
    let doc = |summary: String| doc_block(&summary, &field.docs);

    match field.parsed_field_type.parsed_type.as_ref() {
        Some(ParsedType::OptionalType(ga_type_box)) => {
            let parsed_ga_field_type: &ParsedFieldType = ga_type_box;
            let ga_type = &parsed_ga_field_type.field_type;

            let set_doc = doc(format!("Sets `{shown_name}` to `Some(value)`."));
            let reset_doc = doc(format!("Sets `{shown_name}` to `None`."));
            let mut_opt_doc = doc(format!("Sets `{shown_name}` to the given `Option`."));
            let with_doc = doc(format!(
                "Returns `self` with `{shown_name}` set to `Some(value)`."
            ));
            let without_doc = doc(format!("Returns `self` with `{shown_name}` set to `None`."));
            let opt_doc = doc(format!(
                "Returns `self` with `{shown_name}` set to the given `Option`."
            ));

            quote! {
                #set_doc
                #[inline]
                #field_visibility fn #set_field_name(&mut self, value : #ga_type) -> &mut Self {
                    self.#field_name = Some(value);
                    self
                }

                #reset_doc
                #[inline]
                #field_visibility fn #reset_field_name(&mut self) -> &mut Self {
                    self.#field_name = None;
                    self
                }

                #mut_opt_doc
                #[inline]
                #field_visibility fn #mut_opt_field_name(&mut self, value : #field_type) -> &mut Self {
                    self.#field_name = value;
                    self
                }

                #with_doc
                #[inline]
                #field_visibility fn #with_field_name(self, value : #ga_type) -> Self {
                    Self {
                        #field_name : Some(value),
                        .. self
                    }
                }

                #without_doc
                #[inline]
                #field_visibility fn #without_field_name(self) -> Self {
                    Self {
                        #field_name : None,
                        .. self
                    }
                }

                #opt_doc
                #[inline]
                #field_visibility fn #opt_field_name(self, value : #field_type) -> Self {
                    Self {
                        #field_name : value,
                        .. self
                    }
                }
            }
        }
        _ => {
            let set_doc = doc(format!("Sets `{shown_name}` to `value`."));
            let with_doc = doc(format!(
                "Returns `self` with `{shown_name}` set to `value`."
            ));

            quote! {
                #set_doc
                #[inline]
                #field_visibility fn #set_field_name(&mut self, value : #field_type) -> &mut Self {
                    self.#field_name = value;
                    self
                }

                #with_doc
                #[inline]
                #field_visibility fn #with_field_name(self, value : #field_type) -> Self {
                    Self {
                        #field_name : value,
                        .. self
                    }
                }
            }
        }
    }
}

fn generate_factory_method(
    struct_name: &Ident,
    fields: &Vec<ParsedField>,
) -> proc_macro2::TokenStream {
    let required_fields: Vec<ParsedField> = fields
        .clone()
        .into_iter()
        .filter(|f| f.is_required_field())
        .collect();

    let generated_new_params = generate_new_params(&required_fields);
    let generated_factory_assignments = generate_factory_assignments(fields);
    let new_doc = doc_block(
        &format!(
            "Creates a new `{}` from its required fields. `Option` fields start as `None`, \
             and fields with `#[default]` take their default.",
            struct_name.unraw()
        ),
        &[],
    );

    quote! {
        #new_doc
        pub fn new(#(#generated_new_params)*) -> Self {
            Self {
                #(#generated_factory_assignments)*
            }
        }
    }
}

fn generate_new_params(fields: &[ParsedField]) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|f| {
            let param_name = &f.ident;
            let param_type = &f.parsed_field_type.field_type;

            quote! {
                #param_name : #param_type,
            }
        })
        .collect()
}

fn generate_factory_assignments(fields: &[ParsedField]) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|f| {
            let param_name = &f.ident;
            if let Some(param_default_value) = f.default_tokens.as_ref() {
                quote! {
                    #param_name : #param_default_value,
                }
            } else if f.is_option() {
                quote! {
                    #param_name : None,
                }
            } else {
                quote! {
                    #param_name : #param_name,
                }
            }
        })
        .collect()
}

fn generate_init_struct(
    struct_name: &Ident,
    fields: &Vec<ParsedField>,
    struct_generic_params: &Vec<&TypeParam>,
    struct_generic_params_idents: &Vec<&Ident>,
    struct_lifetime_params: &Vec<&LifetimeParam>,
    struct_where_decl: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    let init_struct_name = format_ident!("{}Init", struct_name);

    let required_fields: Vec<ParsedField> = fields
        .clone()
        .into_iter()
        .filter(|f| f.is_required_field())
        .collect();

    let generated_init_fields = generate_init_fields(struct_name, &required_fields);
    let init_struct_doc = doc_block(
        &format!(
            "Required fields of `{0}`; convert with `{0}::from` or `.into()`.",
            struct_name.unraw()
        ),
        &[],
    );
    let generated_init_new_params = generate_init_new_params(&required_fields);

    let mut init_fields_generic_params: Vec<&&TypeParam> = required_fields
        .iter()
        .filter_map(|f| {
            struct_generic_params
                .iter()
                .find(|gp| field_contains_type(&f.parsed_field_type.field_type, gp))
        })
        .collect();

    init_fields_generic_params.dedup_by_key(|tp| &tp.ident);

    let init_fields_generic_params_idents: Vec<&Ident> = init_fields_generic_params
        .iter()
        .map(|gp| &gp.ident)
        .collect();

    let struct_generic_where_decl: proc_macro2::TokenStream = struct_where_decl
        .as_ref()
        .map_or(quote! {}, |wh| quote! { #wh });

    let mut init_fields_lifetime_params: Vec<&&LifetimeParam> = required_fields
        .iter()
        .filter_map(|f| {
            struct_lifetime_params
                .iter()
                .find(|lt| field_contains_lifetime(f, lt))
        })
        .collect();

    init_fields_lifetime_params.dedup_by_key(|lt| &lt.lifetime.ident);

    if init_fields_generic_params.is_empty() && init_fields_lifetime_params.is_empty() {
        let struct_name_with_possible_generics_lt =
            if struct_generic_params.is_empty() && struct_lifetime_params.is_empty() {
                quote! {
                    #struct_name
                }
            } else {
                quote! {
                   #struct_name<'_>
                }
            };

        quote! {
            #init_struct_doc
            #[allow(dead_code)]
            #[allow(clippy::needless_update)]
            pub struct #init_struct_name {
                #(#generated_init_fields)*
            }

            #[allow(clippy::needless_update)]
            impl From <#init_struct_name> for #struct_name_with_possible_generics_lt {
                 fn from(value: #init_struct_name) -> Self {
                    #struct_name::new(
                        #(#generated_init_new_params)*
                    )
                 }
            }
        }
    } else {
        quote! {
            #init_struct_doc
            #[allow(dead_code)]
            #[allow(clippy::needless_update)]
            pub struct #init_struct_name< #(#init_fields_lifetime_params),* #(#init_fields_generic_params),* > {
                #(#generated_init_fields)*
            }

            #[allow(clippy::needless_update)]
            impl < #(#struct_lifetime_params),* #(#struct_generic_params),* > From < #init_struct_name< #(#init_fields_lifetime_params),* #(#init_fields_generic_params_idents),* > > for #struct_name< #(#struct_lifetime_params),* #(#struct_generic_params_idents),* > #struct_generic_where_decl {
                  fn from(value: #init_struct_name< #(#init_fields_lifetime_params),* #(#init_fields_generic_params_idents),*> ) -> Self {
                    #struct_name::new(
                        #(#generated_init_new_params)*
                    )
                 }
            }
        }
    }
}

fn generate_init_fields(
    struct_name: &Ident,
    fields: &Vec<ParsedField>,
) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|f| {
            let param_name = &f.ident;
            let param_type = &f.parsed_field_type.field_type;
            let field_doc = doc_block(
                &format!(
                    "Value for the `{}` field of `{}`.",
                    param_name.unraw(),
                    struct_name.unraw()
                ),
                &f.docs,
            );

            quote! {
                #field_doc
                pub #param_name : #param_type,
            }
        })
        .collect()
}

fn generate_init_new_params(fields: &Vec<ParsedField>) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|f| {
            let param_name = &f.ident;
            quote! {
                value.#param_name,
            }
        })
        .collect()
}

fn parse_field_default_attr(field: &Field) -> Option<proc_macro2::TokenStream> {
    field
        .attrs
        .iter()
        .find(|a| matches!(a.style, AttrStyle::Outer) && a.path().is_ident("default"))
        .and_then(|a| match &a.meta {
            Meta::NameValue(MetaNameValue {
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }),
                ..
            }) => Some(
                // An unparsable default becomes a compile error spanned on the
                // attribute's string, in place of the default expression.
                s.parse::<proc_macro2::TokenStream>()
                    .unwrap_or_else(|e| e.to_compile_error()),
            ),
            // Dropping `#[default = 10]` would silently make the field
            // required, so it is an error in place of the default. The path
            // and list forms stay ignored: other derives (SmartDefault, for
            // one) share the `default` attribute name.
            Meta::NameValue(_) => Some(
                Error::new_spanned(
                    a,
                    "expected a string literal: `#[default = \"<expression>\"]`",
                )
                .to_compile_error(),
            ),
            _ => None,
        })
}

fn field_contains_type(field_type: &Type, tp: &TypeParam) -> bool {
    match field_type {
        Type::Path(ref path) => path.path.segments.iter().any(|s| {
            s.ident.eq(&tp.ident)
                || match s.arguments {
                    PathArguments::AngleBracketed(ref params) => {
                        params.args.iter().any(|ga| match ga {
                            GenericArgument::Type(ref ty) => field_contains_type(ty, tp),
                            _ => false,
                        })
                    }
                    _ => false,
                }
        }),
        _ => false,
    }
}

fn field_contains_lifetime(field: &ParsedField, lt: &LifetimeParam) -> bool {
    field
        .parsed_field_type
        .lifetime
        .as_ref()
        .filter(|flt| lt.lifetime.eq(flt))
        .is_some()
        || field_contains_lifetime_type(&field.parsed_field_type.field_type, lt)
}

fn field_contains_lifetime_type(field_type: &Type, lt: &LifetimeParam) -> bool {
    match field_type {
        Type::Path(ref path) => path.path.segments.iter().any(|s| match s.arguments {
            PathArguments::AngleBracketed(ref params) => params.args.iter().any(|ga| match ga {
                GenericArgument::Type(ref ty) => field_contains_lifetime_type(ty, lt),
                GenericArgument::Lifetime(ref flt) => lt.lifetime.eq(flt),
                _ => false,
            }),
            _ => false,
        }),
        _ => false,
    }
}
