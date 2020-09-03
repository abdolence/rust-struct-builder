[![Cargo](https://img.shields.io/crates/v/rsb_derive.svg)](https://crates.io/crates/rsb_derive)

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
rsb_derive = "0.2"
```

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
      .opt_opt_field1(Some(("hey".into())) // you can use opt_<field> to provide Option<> inputs
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
    .opt_field1("hey".into()) // no prefix with for mutable setter    
    .opt_field2(10)
    .field2(15)
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


## Licence
Apache Software License (ASL)

## Author
Abdulla Abdurakhmanov
