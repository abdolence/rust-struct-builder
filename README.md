[![Cargo](https://img.shields.io/crates/v/rsb_derive.svg)](https://crates.io/crates/rsb_derive)

# Opinionated and Option-based builder pattern macro for Rust

## Motivation
A derive macros to support a builder pattern for Rust:
- Everything except `Option<>` fields in structs are required, so you 
don't need any additional attributes to indicate it, 
and the presence of required params is checked at the compile time (not at the runtime).
- To create new struct instances there is `::new` and an auxiliary init struct definition 
with only required fields (to compensate the Rust's named params inability). 

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rsb_derive = "0.1"
```

```rust
// Import it
use rsb_derive::Builder;

// And use it on your structs
#[derive(Clone,Builder)]
struct MyStructure {
    req_field1: String,
    req_field2: i32,
    opt_field1: Option<String>,
    opt_field2: Option<i32>
};

// Creating instances

// Option #1:
let s1 : SimpleStrValueStruct = SimpleStrValueStruct::new(
            "hey".into(),
            0);

// Option #2 (named arguments emulation):
let s2 : SimpleStrValueStruct = SimpleStrValueStructInit {
        req_field1 : "hey".into(),
        req_field2 : 0
    }.into();


// Working with instances
let updated = 
    s1.clone()
      .with_opt_field1("hey".into()) // for Option<> fields you specify a bare argument
      .without_opt_field2() // you can reset Option<> if you need it
      .with_req_field2(10); // you can update required params as well

// All together example

let s1 : SimpleStrValueStruct =
    SimpleStrValueStruct::from(
        SimpleStrValueStructInit {
            req_field1 : "hey".into(),
            req_field2 : 0
        }
    )
        .with_opt_field1("hey".into())
        .with_opt_field2(10);

``` 

You're free to use the Rust `Default` if you'd like to on your own structs or on auxiliary init structs. 
This macros doesn't interfere with this pattern.

The macros generates the following functions and instances for your structures:
- `with/without_<field_name>` : immutable setters for fields
- `set/reset_<field_name>` : mutable setters for fields
- `new` : factory method with required fields as arguments
- `From<>` instance from an an auxiliary init struct definition with only required fields. 
The init structure generated as `<YourStructureName>Init`. So, you can use `from(...)` or `into()` 
functions from it.

## Licence
Apache Software License (ASL)

## Author
Abdulla Abdurakhmanov
