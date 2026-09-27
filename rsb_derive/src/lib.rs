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
//! ## Documentation
//!
//! Everything the macro generates has doc comments: a summary line, plus the doc comments
//! of the field it works with. So you can use it on public structs in crates
//! with `#![deny(missing_docs)]`:
//!
//! ```
//! /// Connection settings.
//! #[deny(missing_docs)]
//! pub mod settings {
//!     use rsb_derive::Builder;
//!
//!     /// Where and how to connect.
//!     #[derive(Builder)]
//!     pub struct Connection {
//!         /// Host name or IP address to connect to.
//!         pub host: String,
//!         /// User to connect as.
//!         pub user: Option<String>,
//!     }
//! }
//! # fn main() {}
//! ```
//!
//! ## Field names
//!
//! The separate `BuilderFieldNames` derive adds an associated const with the
//! struct's field names in declaration order. It does not need `Builder`.
//!
//! ```
//! use rsb_derive::BuilderFieldNames;
//!
//! #[derive(BuilderFieldNames)]
//! struct Token {
//!     pub r#type: String,
//!     pub text: String,
//! }
//!
//! // Raw identifiers are listed without their `r#` prefix.
//! assert_eq!(Token::FIELD_NAMES, ["type", "text"]);
//! ```
//!
//! Be aware that for generic structs you need to specify the type parameters,
//! even though the names do not depend on them. `Wrapper::FIELD_NAMES` does not compile:
//!
//! ```
//! use rsb_derive::BuilderFieldNames;
//!
//! #[derive(BuilderFieldNames)]
//! struct Wrapper<T> {
//!     pub inner: T,
//! }
//!
//! assert_eq!(Wrapper::<i32>::FIELD_NAMES, ["inner"]);
//! ```
//!
//! ## Example
//!
//! Full example available [here](https://github.com/abdolence/rust-struct-builder/blob/master/rsb_test/examples/builder.rs),
//! you can run it from the repository with `cargo run -p rsb_test --example builder`.
//!
//! Details and source code: <https://github.com/abdolence/rust-struct-builder>
//!

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::*;
use std::collections::HashSet;
use syn::ext::IdentExt;
use syn::visit::{self, Visit};
use syn::*;

#[proc_macro_derive(Builder, attributes(default))]
pub fn struct_builder_macro(input: TokenStream) -> TokenStream {
    let item: syn::Item = syn::parse(input).expect("failed to parse input");
    let span = Span::call_site();
    match item {
        Item::Struct(ref struct_item) => match struct_item.fields {
            Fields::Named(ref named_fields) => {
                let struct_name = &struct_item.ident;
                let generics = &struct_item.generics;
                let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

                let struct_fields = parse_fields(named_fields);

                let generated_factory_method = generate_factory_method(struct_name, &struct_fields);
                let generated_fields_methods = generate_fields_functions(&struct_fields);
                let generated_aux_init_struct =
                    generate_init_struct(struct_name, &struct_fields, generics);

                let output = quote! {
                    #[allow(dead_code)]
                    #[allow(clippy::needless_update)]
                    impl #impl_generics #struct_name #ty_generics #where_clause {
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

#[proc_macro_derive(BuilderFieldNames)]
pub fn struct_field_names_macro(input: TokenStream) -> TokenStream {
    let item: syn::Item = syn::parse(input).expect("failed to parse input");
    field_names_impl(item).into()
}

fn field_names_impl(item: syn::Item) -> proc_macro2::TokenStream {
    let span = Span::call_site();
    match item {
        Item::Struct(ref struct_item) => match struct_item.fields {
            Fields::Named(ref named_fields) => {
                let struct_name = &struct_item.ident;
                let (impl_generics, ty_generics, where_clause) =
                    struct_item.generics.split_for_impl();

                let field_names: Vec<String> = named_fields
                    .named
                    .iter()
                    .filter_map(|f| f.ident.as_ref())
                    .map(|ident| ident.unraw().to_string())
                    .collect();
                let field_count = field_names.len();

                let names_doc = doc_block(
                    &format!(
                        "Names of the fields of `{}`, in declaration order.",
                        struct_name.unraw()
                    ),
                    &[],
                );

                quote! {
                    #[allow(dead_code)]
                    impl #impl_generics #struct_name #ty_generics #where_clause {
                        #names_doc
                        pub const FIELD_NAMES: [&'static str; #field_count] = [#(#field_names),*];
                    }
                }
            }
            _ => Error::new(
                span,
                "BuilderFieldNames works only on structs with named fields",
            )
            .to_compile_error(),
        },
        _ => Error::new(span, "BuilderFieldNames derive works only on structs").to_compile_error(),
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
    docs: Vec<LitStr>,
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

/// The field's doc comments as they are copied onto the items generated for
/// it, without their fenced code blocks: rustdoc runs every code block of
/// every item as a doctest, so a copied example would run once per generated
/// item. The field itself keeps its docs whole.
///
/// Doc comments arrive as one `#[doc]` attribute per line (or one per block
/// comment), so an open fence is tracked across attributes. A doc value that
/// is not a string literal, such as `include_str!(..)`, cannot be inspected
/// for examples and is not copied. List forms such as `#[doc(hidden)]` are not
/// documentation text and stay on the field alone.
fn parse_field_docs(field: &Field) -> Vec<LitStr> {
    let mut open_fence: Option<CodeFence> = None;
    let docs: Vec<LitStr> = field
        .attrs
        .iter()
        .filter(|a| matches!(a.style, AttrStyle::Outer) && a.path().is_ident("doc"))
        .filter_map(|a| match &a.meta {
            Meta::NameValue(MetaNameValue {
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }),
                ..
            }) => Some(s),
            _ => None,
        })
        .filter_map(|s| {
            let text = s.value();
            let kept: Vec<&str> = text
                .split('\n')
                .filter(|line| CodeFence::keeps_line(&mut open_fence, line))
                .collect();
            (!kept.is_empty()).then(|| LitStr::new(&kept.join("\n"), s.span()))
        })
        .collect();

    if docs.iter().all(|s| s.value().trim().is_empty()) {
        Vec::new()
    } else {
        docs
    }
}

/// An open CommonMark code fence: a run of at least three backticks or
/// tildes, closed by a run of the same character at least as long.
struct CodeFence {
    marker: char,
    len: usize,
}

impl CodeFence {
    /// A fence opening or closing on `line`, with the text after it (the info
    /// string of an opening fence). Indentation is not limited to the three
    /// spaces CommonMark allows, so a fence nested in a list item counts too.
    fn parse(line: &str) -> Option<(Self, &str)> {
        let trimmed = line.trim_start();
        let marker = trimmed.chars().next().filter(|c| matches!(c, '`' | '~'))?;
        let rest = trimmed.trim_start_matches(marker);
        let len = trimmed.len() - rest.len();
        (len >= 3).then_some((Self { marker, len }, rest))
    }

    /// Advances the fence state over `line` and says whether the line is
    /// outside every code block, fence lines included.
    fn keeps_line(open: &mut Option<Self>, line: &str) -> bool {
        match (open.as_ref(), Self::parse(line)) {
            (None, Some((fence, info))) => {
                // A backtick run followed by another backtick on the line is
                // inline code, not a fence.
                if fence.marker == '`' && info.contains('`') {
                    return true;
                }
                *open = Some(fence);
                false
            }
            (None, None) => true,
            (Some(current), Some((fence, rest))) => {
                if fence.marker == current.marker
                    && fence.len >= current.len
                    && rest.trim().is_empty()
                {
                    *open = None;
                }
                false
            }
            (Some(_), None) => false,
        }
    }
}

/// Doc attributes for a generated item: `summary` as the first paragraph,
/// followed by the field's own doc comments as a separate paragraph.
fn doc_block(summary: &str, field_docs: &[LitStr]) -> proc_macro2::TokenStream {
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
            #(#[doc = #field_docs])*
        }
    }
}

fn generate_fields_functions(fields: &[ParsedField]) -> Vec<proc_macro2::TokenStream> {
    fields.iter().map(generate_field_functions).collect()
}

fn generate_field_functions(field: &ParsedField) -> proc_macro2::TokenStream {
    let field_name = &field.ident;
    // The mutable setter is the field's own identifier, raw prefix included:
    // `format_ident!` strips `r#`, which would leave a keyword such as `type`
    // as the method name. The prefixed names below rely on that stripping.
    let set_field_name = field_name;
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
    generics: &Generics,
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

    let init_generics = init_struct_generics(&required_fields, generics);
    let (_, init_ty_generics, _) = init_generics.split_for_impl();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        #init_struct_doc
        #[allow(dead_code)]
        #[allow(clippy::needless_update)]
        pub struct #init_struct_name #init_generics {
            #(#generated_init_fields)*
        }

        #[allow(clippy::needless_update)]
        impl #impl_generics From<#init_struct_name #init_ty_generics> for #struct_name #ty_generics #where_clause {
            fn from(value: #init_struct_name #init_ty_generics) -> Self {
                #struct_name::new(
                    #(#generated_init_new_params)*
                )
            }
        }
    }
}

/// The generic parameters of `<S>Init`: those of the struct that its required
/// fields use, lifetimes first.
///
/// The list is public API, and its order must not change for any struct that
/// already compiles. That order comes from a per-field search: for each
/// required field in turn, the first declared lifetime and the first declared
/// type parameter that `field_contains_lifetime` and `field_contains_type`
/// find in it. Parameters the search misses, such as a second parameter in one
/// field, a const parameter or one inside an array, follow in declaration
/// order. Listing everything in declaration order instead would turn
/// `SInit<B, A>` of `S<A, B> { b: B, a: A }` into `SInit<A, B>`.
///
/// Bounds are kept and the struct's `where` clause is not. Defaults are
/// dropped, because a parameter with a default must come after every one
/// without, which the field order does not respect.
fn init_struct_generics(required_fields: &[ParsedField], generics: &Generics) -> Generics {
    let params: Vec<&GenericParam> = generics.params.iter().collect();
    let is_lifetime = |i: &usize| matches!(params[*i], GenericParam::Lifetime(_));
    let mut chosen: Vec<usize> = Vec::new();

    for field in required_fields {
        let first_lifetime = params.iter().position(|p| match p {
            GenericParam::Lifetime(lt) => field_contains_lifetime(field, lt),
            _ => false,
        });
        let first_type = params.iter().position(|p| match p {
            GenericParam::Type(tp) => field_contains_type(&field.parsed_field_type.field_type, tp),
            _ => false,
        });
        for i in first_lifetime.into_iter().chain(first_type) {
            if !chosen.contains(&i) {
                chosen.push(i);
            }
        }
    }

    let mut used = UsedNames::default();
    for field in required_fields {
        used.visit_type(&field.parsed_field_type.field_type);
    }
    for (i, param) in params.iter().enumerate() {
        let is_used = match param {
            GenericParam::Lifetime(lt) => used.lifetimes.contains(&lt.lifetime.ident),
            GenericParam::Type(TypeParam { ident, .. })
            | GenericParam::Const(ConstParam { ident, .. }) => used.paths.contains(ident),
        };
        if is_used && !chosen.contains(&i) {
            chosen.push(i);
        }
    }

    let (lifetimes, types_and_consts): (Vec<usize>, Vec<usize>) =
        chosen.into_iter().partition(is_lifetime);
    let params = lifetimes
        .into_iter()
        .chain(types_and_consts)
        .map(|i| match params[i] {
            GenericParam::Type(tp) => GenericParam::Type(TypeParam {
                default: None,
                ..tp.clone()
            }),
            GenericParam::Const(cp) => GenericParam::Const(ConstParam {
                default: None,
                ..cp.clone()
            }),
            lt @ GenericParam::Lifetime(_) => lt.clone(),
        })
        .collect();

    Generics {
        params,
        ..Generics::default()
    }
}

/// The names a type can refer to a generic parameter by: every lifetime in
/// it, and the first segment of every path that does not start with `::`,
/// which is where a type or const parameter's name appears (`T`, `T::Item`,
/// the `N` of `[u8; N]` or `Foo<N>`). Types inside macro invocations are not
/// seen.
#[derive(Default)]
struct UsedNames {
    lifetimes: HashSet<Ident>,
    paths: HashSet<Ident>,
}

impl<'ast> Visit<'ast> for UsedNames {
    fn visit_lifetime(&mut self, lifetime: &'ast Lifetime) {
        self.lifetimes.insert(lifetime.ident.clone());
    }

    fn visit_path(&mut self, path: &'ast Path) {
        if path.leading_colon.is_none() {
            if let Some(first) = path.segments.first() {
                self.paths.insert(first.ident.clone());
            }
        }
        visit::visit_path(self, path);
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
            // `Self` in these docs means the struct they were written on, but
            // here they sit on `<S>Init`, where it would resolve to the init
            // struct and the link would break.
            let struct_path = struct_name.to_string();
            let docs: Vec<LitStr> = f
                .docs
                .iter()
                .map(|s| LitStr::new(&rewrite_self_links(&s.value(), &struct_path), s.span()))
                .collect();
            let field_doc = doc_block(
                &format!(
                    "Value for the `{}` field of `{}`.",
                    param_name.unraw(),
                    struct_name.unraw()
                ),
                &docs,
            );

            quote! {
                #field_doc
                pub #param_name : #param_type,
            }
        })
        .collect()
}

/// `text` with `Self` replaced by `target` wherever `Self` starts the target
/// of a Markdown link, which is where rustdoc resolves intra-doc links:
/// `[Self::x]`, `` [`Self::x`] ``, `[text](Self::x)`, `[text][Self::x]` and a
/// reference definition `[label]: Self::x`. `Self` elsewhere, in prose or a
/// code span, is left alone, as is a word merely starting with `Self`.
fn rewrite_self_links(text: &str, target: &str) -> String {
    const SELF: &str = "Self";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find(SELF) {
        let (before, from_self) = rest.split_at(pos);
        let after = &from_self[SELF.len()..];
        out.push_str(before);
        let ends_path_segment =
            after.is_empty() || after.starts_with("::") || after.starts_with([']', '`', ')', '>']);
        if ends_path_segment && starts_link_target(&out) {
            out.push_str(target);
        } else {
            out.push_str(SELF);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Whether `before` ends where a link target begins: after an unescaped `[`,
/// after `](` or `]:`, allowing the backticks, `<` and spaces that may
/// precede the path.
fn starts_link_target(before: &str) -> bool {
    let before = before.trim_end_matches(['`', '<', ' ']);
    match before.strip_suffix('[') {
        Some(prefix) => !prefix.ends_with('\\'),
        None => before.ends_with("](") || before.ends_with("]:"),
    }
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
            // A string holds the default expression as source text; an
            // unparsable one becomes a compile error spanned on the string.
            Meta::NameValue(MetaNameValue {
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }),
                ..
            }) => Some(
                s.parse::<proc_macro2::TokenStream>()
                    .unwrap_or_else(|e| e.to_compile_error()),
            ),
            // Any other value is the default itself: `#[default = 100]` is
            // `100`. This is also how SmartDefault reads the same attribute,
            // so a field carrying both derives gets one default from each.
            Meta::NameValue(MetaNameValue { value, .. }) => Some(value.to_token_stream()),
            // `#[default]` and `#[default(...)]` belong to other derives
            // (SmartDefault's list form, for one) and leave the field as is.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn field_names_error(input: &str) -> String {
        field_names_impl(syn::parse_str(input).expect("test input is a valid item")).to_string()
    }

    /// The doc lines `parse_field_docs` copies from the only field of `src`.
    fn copied_doc_lines(src: &str) -> Vec<String> {
        let item: ItemStruct = syn::parse_str(src).expect("test input is a valid struct");
        let field = item.fields.iter().next().expect("test struct has a field");
        parse_field_docs(field)
            .iter()
            .flat_map(|s| s.value().split('\n').map(str::to_owned).collect::<Vec<_>>())
            .collect()
    }

    #[test]
    fn fenced_blocks_are_not_copied() {
        let src = "struct S {
            /// Before.
            ///
            /// ```
            /// let x = 1;
            /// ```
            ///
            /// Between.
            /// ~~~rust,no_run
            /// let y = 2;
            /// ~~~
            /// After.
            x: i32,
        }";
        assert_eq!(
            copied_doc_lines(src),
            [" Before.", "", "", " Between.", " After."]
        );
    }

    #[test]
    fn a_fence_closes_only_on_its_own_marker() {
        let src = "struct S {
            /// Text.
            /// ````markdown
            /// ```
            /// ~~~
            /// ````
            /// Kept.
            x: i32,
        }";
        assert_eq!(copied_doc_lines(src), [" Text.", " Kept."]);
    }

    #[test]
    fn fences_are_tracked_inside_a_block_doc_comment() {
        let src = "struct S {
            /** Text.
            ```
            let x = 1;
            ```
            Kept. */
            x: i32,
        }";
        let lines = copied_doc_lines(src);
        assert!(
            lines
                .iter()
                .all(|l| !l.contains("```") && !l.contains("let x")),
            "{lines:?}"
        );
        assert!(lines.iter().any(|l| l.contains("Kept.")), "{lines:?}");
    }

    #[test]
    fn an_unclosed_fence_runs_to_the_end_of_the_docs() {
        let src = "struct S {
            /// Text.
            /// ```
            /// let x = 1;
            x: i32,
        }";
        assert_eq!(copied_doc_lines(src), [" Text."]);
    }

    #[test]
    fn inline_code_is_not_a_fence() {
        let src = "struct S {
            /// ``` inline ``` and `code`.
            /// Kept.
            x: i32,
        }";
        assert_eq!(
            copied_doc_lines(src),
            [" ``` inline ``` and `code`.", " Kept."]
        );
    }

    #[test]
    fn docs_that_are_only_a_code_block_are_not_copied() {
        let src = "struct S {
            ///
            /// ```
            /// let x = 1;
            /// ```
            ///
            x: i32,
        }";
        assert!(copied_doc_lines(src).is_empty());
    }

    #[test]
    fn non_literal_docs_are_not_copied() {
        let src = r#"struct S {
            /// Kept.
            #[doc = concat!("Not ", "inspected.")]
            #[doc(hidden)]
            x: i32,
        }"#;
        assert_eq!(copied_doc_lines(src), [" Kept."]);
    }

    #[test]
    fn self_links_point_at_the_struct() {
        for (doc, expected) in [
            ("Uses [`Self::helper`].", "Uses [`Job::helper`]."),
            ("Uses [Self::helper].", "Uses [Job::helper]."),
            ("See [this](Self::helper).", "See [this](Job::helper)."),
            ("See [this](<Self::helper>).", "See [this](<Job::helper>)."),
            ("See [this][Self::helper].", "See [this][Job::helper]."),
            ("Part of [`Self`] and [Self].", "Part of [`Job`] and [Job]."),
            ("[h]: Self::helper", "[h]: Job::helper"),
            ("[a](Self::a), [b](Self::b)", "[a](Job::a), [b](Job::b)"),
        ] {
            assert_eq!(rewrite_self_links(doc, "Job"), expected);
        }
    }

    #[test]
    fn self_outside_link_targets_is_kept() {
        for doc in [
            "Self::helper in prose, and `Self::helper` in code.",
            "Links to [SelfDescribing] and [Self-evident truths].",
            "An escaped \\[Self::helper] is not a link.",
            "[see Self::helper] is link text, not a target.",
        ] {
            assert_eq!(rewrite_self_links(doc, "Job"), doc);
        }
    }

    #[test]
    fn init_fields_link_to_the_struct_and_methods_keep_self() {
        let item: ItemStruct = syn::parse_str(
            "struct Job {
                /// Checked by [`Self::helper`].
                name: String,
            }",
        )
        .expect("test input is a valid struct");
        let Fields::Named(named) = &item.fields else {
            unreachable!("test struct has named fields")
        };
        let fields = parse_fields(named);
        let init_fields = generate_init_fields(&item.ident, &fields);
        let init = quote!(#(#init_fields)*).to_string();
        assert!(init.contains("[`Job::helper`]"), "{init}");
        let methods = generate_field_functions(&fields[0]).to_string();
        assert!(methods.contains("[`Self::helper`]"), "{methods}");
    }

    #[test]
    fn field_names_errors_name_their_derive() {
        for input in ["enum E { A }", "struct T(i32);", "struct U;"] {
            let error = field_names_error(input);
            assert!(error.contains("compile_error"), "{input}: {error}");
            assert!(error.contains("BuilderFieldNames"), "{input}: {error}");
        }
    }
}
