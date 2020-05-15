//! Rust struct builder implementation macro
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::*;
use syn::*;



#[proc_macro_derive(Builder)]
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
    parsed_field_type : ParsedFieldType
}

impl ParsedField {
    fn is_option(&self) -> bool {
        self.parsed_field_type.parsed_type.as_ref().filter(|t| t.is_option()).is_some()
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
                parsed_type : parsed_type
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
        parsed_field_type : parse_field_type(&field.ty)
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

fn generate_fields_functions(fields : &Vec<ParsedField>) -> Vec<proc_macro2::TokenStream> {
    fields.iter().map(generate_field_functions).collect()
}

fn generate_field_functions(field : &ParsedField) -> proc_macro2::TokenStream {
    let field_name = &field.ident;
    let set_field_name = format_ident!("{}",field_name);
    let reset_field_name = format_ident!("reset_{}",field_name);
    let with_field_name = format_ident!("with_{}",field_name);
    let without_field_name = format_ident!("without_{}",field_name);

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
            .filter(|f| !f.is_option())
            .collect();

    let generated_new_params = generate_new_params(&required_fields);
    let generated_factory_assignments = generated_factory_assignments(&fields);

    quote! {
        pub fn new(#(#generated_new_params)*) -> Self {
            Self {
                #(#generated_factory_assignments)*
            }
        }
    }
}

fn generate_new_params(fields : &Vec<ParsedField>) -> Vec<proc_macro2::TokenStream> {
    fields
        .into_iter()
        .map(|f| {
            let param_name = &f.ident;
            let param_type = &f.parsed_field_type.field_type;

            quote! {
                #param_name : #param_type,
            }
        })
        .collect()
}

fn generated_factory_assignments(fields : &Vec<ParsedField>) -> Vec<proc_macro2::TokenStream> {
    fields
        .into_iter()
        .map(|f| {
            let param_name = &f.ident;
            if f.is_option() {
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
            .filter(|f| !f.is_option())
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
            struct #init_struct_name {
                #(#generated_init_fields)*
            }

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
            struct #init_struct_name< #(#init_fields_generic_params),* > {
                #(#generated_init_fields)*
            }

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
        .into_iter()
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