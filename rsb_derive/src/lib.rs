#![allow(clippy::ptr_arg)]

//! Rust struct builder implementation macro
//!
//! ## Motivation
//! A derive macros to support a builder pattern for Rust:
//! - Everything except `Option<>` fields and explicitly defined `default` attribute in structs are required, so you
//! don't need any additional attributes to indicate it, and the presence of required params
//! is checked at the compile time (not at the runtime).
//! - To create new struct instances there is `::new` and an auxiliary init struct definition
//! with only required fields (to compensate the Rust's named params inability).
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
//!     req_field1: String,
//!     req_field2: i32,
//!     opt_field1: Option<String>,
//!     opt_field2: Option<i32>
//! }
//!
//! let s1 : SimpleStrValueStruct =
//!     SimpleStrValueStruct::from(
//!         SimpleStrValueStructInit {
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
//! The init structure generated as `<YourStructureName>Init`. So, you can use `from(...)` or `into()`
//! functions from it.
//!
//! ## Defaults
//!
//! ```
//! #[derive(Debug, Clone, PartialEq, Builder)]
//! struct StructWithDefault {
//!     req_field1: String,
//!     #[default="10"]
//!     req_field2: i32, // default here make this field behave like optional
//!
//!     opt_field1: Option<String>,
//!     #[default="Some(11)"]
//!     opt_field2: Option<i32> // default works also on optional fields
//! }
//! ```
//!
//! Details and source code: [https://github.com/abdolence/rust-struct-builder]: https://github.com/abdolence/rust-struct-builder
//!

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::*;
use syn::*;
use std::ops::Index;


#[proc_macro_derive(Builder, attributes(default))]
pub fn struct_builder_macro(input: TokenStream) -> TokenStream {
    let item: syn::Item = syn::parse(input).expect("failed to parse input");
    let span = Span::call_site();
    match item {
        Item::Struct(ref struct_item) => match struct_item.fields {
            Fields::Named(ref named_fields) => {
                let struct_name = &struct_item.ident;
                let struct_generic_params: Vec<&TypeParam> =
                    struct_item.generics.params.iter().map( |ga| {
                        match ga {
                            GenericParam::Type(ref ty) => Some(ty),
                            _ => None
                        }
                    }).flatten().collect();

                let struct_generic_params_idents : Vec<&Ident> = struct_generic_params.iter().map(|gp| &gp.ident).collect();

                let struct_generic_where_decl  : proc_macro2::TokenStream =
                    struct_item.generics.where_clause.as_ref().map_or(quote! {}, |wh| quote! { #wh });

                let struct_fields = parse_fields(&named_fields);

                let generated_factory_method = generate_factory_method(&struct_fields);
                let generated_fields_methods = generate_fields_functions(&struct_fields);

                let generated_aux_init_struct = generate_init_struct(
                    &struct_name,
                    &struct_fields,
                    &struct_generic_params,
                    &struct_generic_params_idents,
                    struct_item.generics.where_clause.as_ref()
                );

                let struct_decl : proc_macro2::TokenStream =
                    if struct_generic_params.is_empty() {
                        quote! {
                            impl #struct_name
                        }
                    }
                    else {
                        quote! {
                            impl< #(#struct_generic_params),* > #struct_name < #(#struct_generic_params_idents),* > #struct_generic_where_decl
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
            _ => Error::new(
                span,
                "Builder works only on the structs with named fields",
            )
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
    OptionalType(Box<ParsedFieldType>)
}

impl ParsedType {
    fn is_option(&self) -> bool {
        match self {
            ParsedType::OptionalType(_) => true,
            _ => false
        }
    }
}

#[derive(Clone)]
struct ParsedFieldType {
    field_type : Type,
    parsed_type : Option<ParsedType>
}


#[derive(Clone)]
struct ParsedField {
    ident : Ident,
    parsed_field_type : ParsedFieldType,
    default_tokens : Option<proc_macro2::TokenStream>
}

impl ParsedField {
    fn is_option(&self) -> bool {
        self.parsed_field_type.parsed_type.as_ref().filter(|t| t.is_option()).is_some()
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
                            params.args.first().map( |ga| {
                                match ga {
                                    GenericArgument::Type(ref ty) => Some(ParsedType::OptionalType(Box::from(parse_field_type(ty)))),
                                    _ => None
                                }
                            }).flatten()
                        }
                        _ => None
                    }

                }
                "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
                | "u128" | "usize" => Some(ParsedType::ScalarType),
                _ => None
            };

            ParsedFieldType {
                field_type : field_type.clone(),
                parsed_type
            }
        }
        _ =>
            ParsedFieldType {
                field_type: field_type.clone(),
                parsed_type : None
            }
    }
}

fn parse_fields(fields : &FieldsNamed) -> Vec<ParsedField> {
    fields.named.iter().map(parse_field).collect()
}

fn parse_field(field : &Field) -> ParsedField {

    ParsedField {
        ident : field.ident.as_ref().unwrap().clone(),
        parsed_field_type : parse_field_type(&field.ty),
        default_tokens : parse_field_default_attr(&field)
    }
}

fn field_contains_type(field_type : &Type, tp  : &TypeParam ) -> bool {
    match field_type {
        Type::Path(ref path) => {
            path.path.segments.iter().any(|s| {
                s.ident.eq(&tp.ident) ||
                    match s.arguments {
                        PathArguments::AngleBracketed(ref params) => {
                            params.args.iter().any(|ga| {
                                match ga {
                                    GenericArgument::Type(ref ty) => field_contains_type(&ty,&tp),
                                    _ => false
                                }
                            })
                        }
                        _ => false
                    }
            })
        }
        _ => false
    }

}

fn generate_fields_functions(fields : &[ParsedField]) -> Vec<proc_macro2::TokenStream> {
    fields.iter().map(generate_field_functions).collect()
}

fn generate_field_functions(field : &ParsedField) -> proc_macro2::TokenStream {
    let field_name = &field.ident;
    let set_field_name = format_ident!("{}",field_name);
    let reset_field_name = format_ident!("reset_{}",field_name);
    let with_field_name = format_ident!("with_{}",field_name);
    let without_field_name = format_ident!("without_{}",field_name);
    let opt_field_name = format_ident!("opt_{}",field_name);
    let mut_opt_field_name = format_ident!("mopt_{}",field_name);

    let field_type = &field.parsed_field_type.field_type;

    match field.parsed_field_type.parsed_type.as_ref() {
        Some(ParsedType::OptionalType(ga_type_box)) => {
            let parsed_ga_field_type : &ParsedFieldType = &*ga_type_box;
            let ga_type = &parsed_ga_field_type.field_type;

            quote! {
                #[inline]
                pub fn #set_field_name(&mut self, value : #ga_type) -> &mut Self {
                    self.#field_name = Some(value);
                    self
                }

                #[inline]
                pub fn #reset_field_name(&mut self) -> &mut Self {
                    self.#field_name = None;
                    self
                }

                #[inline]
                pub fn #mut_opt_field_name(&mut self, value : #field_type) -> &mut Self {
                    self.#field_name = value;
                    self
                }

                #[inline]
                pub fn #with_field_name(self, value : #ga_type) -> Self {
                    Self {
                        #field_name : Some(value),
                        .. self
                    }
                }

                #[inline]
                pub fn #without_field_name(self) -> Self {
                    Self {
                        #field_name : None,
                        .. self
                    }
                }

                #[inline]
                pub fn #opt_field_name(self, value : #field_type) -> Self {
                    Self {
                        #field_name : value,
                        .. self
                    }
                }
            }
        }
        _ => {
            quote! {
                #[inline]
                pub fn #set_field_name(&mut self, value : #field_type) -> &mut Self {
                    self.#field_name = value;
                    self
                }

                #[inline]
                pub fn #with_field_name(self, value : #field_type) -> Self {
                    Self {
                        #field_name : value,
                        .. self
                    }
                }
            }
        }
    }

}

fn generate_factory_method(fields : &Vec<ParsedField>) -> proc_macro2::TokenStream {
    let required_fields : Vec<ParsedField> =
        fields
            .clone()
            .into_iter()
            .filter(|f| f.is_required_field())
            .collect();

    let generated_new_params = generate_new_params(&required_fields);
    let generated_factory_assignments = generate_factory_assignments(&fields);

    quote! {
        pub fn new(#(#generated_new_params)*) -> Self {
            Self {
                #(#generated_factory_assignments)*
            }
        }
    }
}

fn generate_new_params(fields : &[ParsedField]) -> Vec<proc_macro2::TokenStream> {
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

fn generate_factory_assignments(fields : &[ParsedField]) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|f| {
            let param_name = &f.ident;
            if f.default_tokens.is_some() {
                let param_default_value = f.default_tokens.as_ref().unwrap();
                quote! {
                    #param_name : #param_default_value,
                }
            } else if f.is_option() {
                quote! {
                    #param_name : None,
                }
            }
            else {
                quote! {
                    #param_name : #param_name,
                }
            }
        })
        .collect()
}


fn generate_init_struct(struct_name : &Ident, fields : &Vec<ParsedField>,
                        struct_generic_params: &Vec<&TypeParam>,
                        struct_generic_params_idents : &Vec<&Ident>,
                        struct_where_decl : Option<&syn::WhereClause>) -> proc_macro2::TokenStream {
    let init_struct_name = format_ident!("{}Init", struct_name);

    let required_fields : Vec<ParsedField> =
        fields
            .clone()
            .into_iter()
            .filter(|f| f.is_required_field())
            .collect();

    let generated_init_fields = generate_init_fields(&required_fields);
    let generated_init_new_params = generate_init_new_params(&required_fields);

    let init_fields_generic_params : Vec<&&TypeParam> =  required_fields.iter().map(|f| {
        struct_generic_params.iter().find(|gp| {
            field_contains_type(&f.parsed_field_type.field_type,gp)
        })
    }).flatten().collect();

    let init_fields_generic_params_idents : Vec<&Ident> = init_fields_generic_params.iter().map(|gp| &gp.ident).collect();

    let struct_generic_where_decl  : proc_macro2::TokenStream =
        struct_where_decl.as_ref().map_or(quote! {}, |wh| quote! { #wh });

    if init_fields_generic_params.is_empty() {
        quote! {
            #[allow(dead_code)]
            #[allow(clippy::needless_update)]
            pub struct #init_struct_name {
                #(#generated_init_fields)*
            }

            #[allow(clippy::needless_update)]
            impl From<#init_struct_name> for #struct_name {
                 fn from(value: #init_struct_name) -> Self {
                    #struct_name::new(
                        #(#generated_init_new_params)*
                    )
                 }
            }
        }
    }
    else {
        quote! {
            #[allow(dead_code)]
            #[allow(clippy::needless_update)]
            pub struct #init_struct_name< #(#init_fields_generic_params),* > {
                #(#generated_init_fields)*
            }

            #[allow(clippy::needless_update)]
            impl < #(#struct_generic_params),* > From< #init_struct_name< #(#init_fields_generic_params_idents),* > > for #struct_name< #(#struct_generic_params_idents),* > #struct_generic_where_decl {
                  fn from(value: #init_struct_name<#(#init_fields_generic_params_idents),*> ) -> Self {
                    #struct_name::new(
                        #(#generated_init_new_params)*
                    )
                 }
            }
        }
    }
}

fn generate_init_fields(fields : &Vec<ParsedField>) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .map(|f| {
            let param_name = &f.ident;
            let param_type = &f.parsed_field_type.field_type;

            quote! {
                pub #param_name : #param_type,
            }
        })
        .collect()
}

fn generate_init_new_params(fields : &Vec<ParsedField>) -> Vec<proc_macro2::TokenStream> {
    fields
        .into_iter()
        .map(|f| {
            let param_name = &f.ident;
            quote! {
                value.#param_name,
            }
        })
        .collect()
}

fn parse_field_default_attr(field : &Field) -> Option<proc_macro2::TokenStream> {
    field.attrs.iter().find(|a| {
        match a.style {
            AttrStyle::Outer => {
                a.path.segments.first().iter().any (|s| s.ident.eq("default"))
            },
            _ => false
        }
    }).and_then(|a| {
       let attr_tokens : &Vec<proc_macro2::TokenTree> = &a.tokens.clone().into_iter().collect();
        if attr_tokens.len() > 1 {
            match attr_tokens.last().unwrap() {
                proc_macro2::TokenTree::Literal(lit) => {
                    let lit_str = format!("{}",lit);
                    let lit_unquoted_str = lit_str.index(1..lit_str.len()-1);
                    let lit_stream : proc_macro2::TokenStream = syn::parse_str(lit_unquoted_str).unwrap();
                    Some(
                        quote! {
                            #lit_stream
                        }
                    )
                }
                _ => None
            }
        }
        else {
            None
        }
    })
}