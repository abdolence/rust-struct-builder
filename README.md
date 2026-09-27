[![Cargo](https://img.shields.io/crates/v/rsb_derive.svg)](https://crates.io/crates/rsb_derive)
[![tests](https://github.com/abdolence/rust-struct-builder/actions/workflows/tests.yml/badge.svg)](https://github.com/abdolence/rust-struct-builder/actions/workflows/tests.yml)

# Opinionated and Option-based builder pattern macro for Rust

## Motivation
A derive macros to support a builder pattern for Rust:
- Everything except `Option<>` fields and explicitly defined `default` attribute in structs are required, so you 
don't need any additional attributes to indicate it, and the presence of required params 
is checked at the compile time (not at the runtime).
- To create new struct instances there is `::new` and an auxiliary init struct definition 
with only required fields (to compensate the Rust's named params inability). 

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rsb_derive = "0.5"
```

The minimum supported Rust version is 1.71.

The macros generates the following functions and instances for your structures:
- `with/without/opt_<field_name>` : immutable setters for fields (`opt` is an additional setter for `Option<>` input argument)
- `<field_name>/reset/mopt_<field_name>` : mutable setters for fields (`mopt` is an additional setter for `Option<>` input argument)
- `new` : factory method with required fields as arguments
- `From<>` instance from an an auxiliary init struct definition with only required fields. 
The init structure generated as `<YourStructureName>Init`. So, you can use `from(...)` or `into()` 
functions from it.

### Marking the derive attribute on your structures:

```rust
// Import it
use rsb_derive::Builder;

// And use it on your structs
#[derive(Clone,Builder)]
struct MyStructure {
    pub req_field1: String,
    pub req_field2: i32,
    pub opt_field1: Option<String>,
    pub opt_field2: Option<i32>
}
```

### Using the builder pattern on your structures 

```rust
// Creating instances

// Option #1:
let s1 : MyStructure = MyStructure::new(
            "hey".into(),
            0);

// Option #2 (named arguments emulation):
let s2 : MyStructure = MyStructureInit {
        req_field1 : "hey".into(),
        req_field2 : 0
    }.into();


// Working with instances
let updated = 
    s1.clone()
      .with_opt_field1("hey".into()) // for Option<> fields you specify a bare argument
      .without_opt_field2() // you can reset Option<> if you need it
      .opt_opt_field1(Some("hey".into())) // you can use opt_<field> to provide Option<> inputs
      .with_req_field2(10); // you can update required params as well

// All together example

let s1 : MyStructure =
    MyStructure::from(
        MyStructureInit {
            req_field1 : "hey".into(),
            req_field2 : 0
        }
    )
        .with_opt_field1("hey".into())
        .with_opt_field2(10);

// Mutable example (in case you really need it)
let mut s1 : MyStructure =
    MyStructure::from(
        MyStructureInit {
            req_field1 : "hey".into(),
            req_field2 : 0
        }
    );

s1
    .opt_field1("hey".into()) // no `with` prefix for mutable setters
    .opt_field2(10)
    .req_field2(15)
    .reset_opt_field2(); // mutable reset function for optional fields
```

### Defaults

While you're free to use the Rust `Default` on your own structs or on auxiliary init structs 
this lib intentionally ignores this approach and gives you an auxiliary `default` attribute 
to manage this like: 

```rust
#[derive(Debug, Clone, PartialEq, Builder)]
struct StructWithDefault {
    pub req_field1: String,
    #[default="10"]
    pub req_field2: i32, // default here make this field behave like optional

    pub opt_field1: Option<String>,
    #[default="Some(11)"]
    pub opt_field2: Option<i32> // default works also on optional fields
}

let my_struct : StructWithDefault = StructWithDefault::from(
    StructWithDefaultInit {
        req_field1 : "test".into()
    }
);
```

The value of `default` is read like this:
- a string holds an expression, so `#[default="10"]` and `#[default="Some(11)"]` are the source code of the value;
- any other literal is the value itself: `#[default = 100]`, `#[default = 12.5]`, `#[default = true]`;
- `#[default]` and `#[default(...)]` are left to other derives, like `SmartDefault`, and the field stays required.

```rust
#[derive(Debug, Clone, PartialEq, Builder)]
struct Settings {
    pub name: String,
    #[default = 100]
    pub limit: u32,
    #[default = true]
    pub enabled: bool,
}

let settings = Settings::new("test".into());
assert_eq!((settings.limit, settings.enabled), (100, true));
```

Be aware this changed in 0.5.2. Before it, a non-string literal like `#[default = true]` was ignored
and the field stayed required. Now such fields are not in `new()` and `Init` anymore, so remove them from these calls.

### Documentation

Everything the macro generates has doc comments: a summary line, plus the doc comments
of the field it works with. So you can use it on public structs in crates
with `#![deny(missing_docs)]`.

Code blocks in field docs stay on the field only and are not copied to the generated items,
otherwise rustdoc would run the same example as a doctest once per item.
`Self::` links in field docs are rewritten on `Init` fields, so they still point to your struct.

### Raw identifiers and generics

Fields with raw identifiers are supported. The mutable setter keeps the `r#` prefix,
other functions drop it:

```rust
#[derive(Debug, Clone, Builder)]
struct Rule {
    pub r#type: String,
    pub r#match: Option<i32>,
}

let mut rule = Rule::new("word".into()).with_type("number".into()).with_match(1);
rule.r#type("text".into()).reset_match();
```

Generic structs work with lifetimes, type parameters, const generics and defaults for them,
also all mixed together:

```rust
#[derive(Debug, Clone, Builder)]
struct Buffer<'a, T, const N: usize = 2> {
    pub name: &'a str,
    pub items: [T; N],
    pub label: Option<String>,
}

let buffer: Buffer<i32> = Buffer::new("buf", [1, 2]).with_label("numbers".into());
```

## Field names

The separate `BuilderFieldNames` derive adds an associated const `FIELD_NAMES` with the
struct's field names in declaration order. It does not need `Builder`, and you can use both on the same struct:

```rust
use rsb_derive::BuilderFieldNames;

#[derive(BuilderFieldNames)]
struct Token {
    pub r#type: String,
    pub text: String,
}

// Raw identifiers are listed without their `r#` prefix
assert_eq!(Token::FIELD_NAMES, ["type", "text"]);
```

Be aware that for generic structs you need to specify the type parameters,
even though the names do not depend on them:

```rust
#[derive(BuilderFieldNames)]
struct Wrapper<T> {
    pub inner: T,
}

assert_eq!(Wrapper::<i32>::FIELD_NAMES, ["inner"]); // `Wrapper::FIELD_NAMES` does not compile
```

## Example

Full example available [here](https://github.com/abdolence/rust-struct-builder/blob/master/rsb_test/examples/builder.rs), you can run it from the repository with:

```sh
cargo run -p rsb_test --example builder
```

## Licence
Apache Software License (ASL)

## Author
Abdulla Abdurakhmanov
