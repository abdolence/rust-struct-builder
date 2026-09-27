#[deny(missing_docs)]
pub mod documented {
    //! Builders for public structs, compiled under `deny(missing_docs)`: every
    //! item the derive generates must carry docs of its own.

    use rsb_derive::Builder;

    /// A network endpoint.
    #[derive(Debug, Clone, PartialEq, Builder)]
    pub struct Endpoint {
        /// Host name or IP address.
        pub host: String,
        /// TCP port.
        ///
        /// Zero is not a valid port.
        pub port: u16,
        /// Request timeout in seconds.
        pub timeout_secs: Option<u64>,
    }

    /// A value with an optional label.
    #[derive(Debug, Clone, PartialEq, Builder)]
    pub struct Tagged<T> {
        /// The wrapped value.
        pub value: T,
        /// Label shown next to the value.
        pub label: Option<String>,
    }

    /// A struct whose fields are not all documented.
    #[derive(Debug, Clone, PartialEq, Builder)]
    pub struct PartlyDocumented {
        /// Display name.
        pub name: String,
        // The allow covers only the field itself; the builder items generated
        // for it are still checked.
        #[allow(missing_docs)]
        pub count: i32,
        #[allow(missing_docs)]
        pub note: Option<String>,
    }

    /// A struct with a defaulted field.
    #[derive(Debug, Clone, PartialEq, Builder)]
    pub struct WithDefault {
        /// Service name.
        pub service: String,
        /// Number of retries before giving up.
        #[default = "3"]
        pub retries: u32,
    }

    /// Retry settings whose field docs carry examples. Each example is a
    /// doctest of the field alone; the items generated for the field copy
    /// its docs without them.
    #[derive(Debug, Clone, PartialEq, Builder)]
    pub struct RetryPolicy {
        /// Delay between attempts, in milliseconds.
        ///
        /// ```
        /// let policy = rsb_test::documented::RetryPolicy::new(10);
        /// assert_eq!(policy.delay_ms, 10);
        /// ```
        pub delay_ms: u64,
        /// Upper bound on attempts; unbounded when `None`.
        ///
        /// ~~~rust,no_run
        /// use rsb_test::documented::RetryPolicy;
        /// let policy = RetryPolicy::new(10).with_max_attempts(3);
        /// assert_eq!(policy.max_attempts, Some(3));
        /// ~~~
        ///
        /// Counts the first attempt too.
        pub max_attempts: Option<u32>,
    }

    /// An HTTP header.
    #[derive(Debug, Clone, PartialEq, rsb_derive::BuilderFieldNames)]
    pub struct Header {
        /// Header name.
        pub name: String,
        /// Header value.
        pub value: String,
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn documented_builders_build() {
            let endpoint: Endpoint = EndpointInit {
                host: "localhost".into(),
                port: 8080,
            }
            .into();
            assert_eq!(endpoint.with_timeout_secs(5).timeout_secs, Some(5));

            let tagged = Tagged::new(1).with_label("one".into());
            assert_eq!(tagged.label.as_deref(), Some("one"));

            let partly = PartlyDocumented::new("n".into(), 1).with_note("x".into());
            assert_eq!(partly.count, 1);

            assert_eq!(WithDefault::new("svc".into()).retries, 3);
        }
    }
}

#[cfg(test)]
mod tests {

    use rsb_derive::Builder;

    #[derive(Debug, Clone, PartialEq, Builder)]
    struct SimpleStrValueStruct {
        pub req_field1: String,
        pub req_field2: i32,
        pub opt_field1: Option<String>,
        pub opt_field2: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Builder)]
    struct GenericValueStruct<T, B> {
        pub gen_field1: T,
        pub gen_field2: T,
        pub opt_gen_field1: Option<T>,
        pub opt_gen_field2: Option<B>,
    }

    #[derive(Debug, Clone, PartialEq, Builder)]
    struct GenericValueStructWithBounds<T: Copy + Clone> {
        pub gen_field1: T,
        pub opt_gen_field1: Option<T>,
        pub opt_gen_field2: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Builder)]
    struct GenericValueStructWithBoundsWhere<T>
    where
        T: Copy + Clone,
    {
        pub gen_field1: T,
        pub opt_gen_field1: Option<T>,
    }

    #[derive(Debug, Clone, PartialEq, Builder)]
    struct StructWithDefault {
        pub req_field1: String,
        #[default = "10"]
        pub req_field2: i32,
        pub opt_field1: Option<String>,
        #[default = "Some(11)"]
        pub opt_field2: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Builder)]
    struct StructWithDifferentAccess {
        pub req_field1: String,
        req_field2: i32,
        pub opt_field1: Option<String>,
        opt_field2: Option<i32>,
    }

    #[derive(Debug, Clone, PartialEq, Builder)]
    pub struct StructWithLifetime<'a> {
        pub req_field: &'a str,
        pub opt_field: Option<&'a str>,
    }

    #[derive(Debug, Clone, PartialEq, Builder)]
    struct StructWithLifetimeAdv<'a> {
        pub req_field1: &'a str,
        pub req_field2: Vec<StructWithLifetime<'a>>,
        pub opt_field: Option<&'a str>,
    }

    #[test]
    fn new_str_value_struct() {
        let s1: SimpleStrValueStruct = SimpleStrValueStruct::new("hey".into(), 0);

        assert_eq!(s1.req_field1, String::from("hey"));
    }

    #[test]
    fn fill_str_value_struct() {
        let s1 = SimpleStrValueStruct {
            req_field1: "hey".into(),
            req_field2: 0,
            opt_field1: None,
            opt_field2: None,
        }
        .opt_field1("hey".into())
        .clone();

        assert_eq!(s1.opt_field1, Some("hey".into()));

        let s1c = SimpleStrValueStruct {
            req_field1: "hey".into(),
            req_field2: 0,
            opt_field1: None,
            opt_field2: None,
        }
        .with_opt_field1("hey2".into());
        assert_eq!(s1c.opt_field1, Some("hey2".into()));

        assert_eq!(s1c.without_opt_field1().opt_field1, None);
    }

    #[test]
    fn into_str_value_struct() {
        let s1: SimpleStrValueStruct = SimpleStrValueStructInit {
            req_field1: "hey".into(),
            req_field2: 0,
        }
        .into();

        let s11 = s1.clone().with_opt_field1("hey".into()).with_req_field2(10);

        assert_eq!(s1.req_field1, String::from("hey"));
        assert_eq!(s11.req_field1, String::from("hey"));
    }

    #[test]
    fn all_together_test() {
        let s1: SimpleStrValueStruct = SimpleStrValueStruct::from(SimpleStrValueStructInit {
            req_field1: "hey".into(),
            req_field2: 0,
        })
        .with_opt_field1("hey".into())
        .with_opt_field2(10);

        assert_eq!(s1.req_field1, String::from("hey"));
        assert_eq!(s1.opt_field1, Some(String::from("hey")));
    }

    #[test]
    fn all_together_mutable_test() {
        let mut s1: SimpleStrValueStruct = SimpleStrValueStruct::from(SimpleStrValueStructInit {
            req_field1: "hey".into(),
            req_field2: 0,
        });

        s1.opt_field1("hey".into())
            .opt_field2(10)
            .reset_opt_field2();

        assert_eq!(s1.req_field1, String::from("hey"));
        assert_eq!(s1.opt_field1, Some(String::from("hey")));
    }

    #[test]
    fn generic_struct_test() {
        let g1: GenericValueStruct<String, i64> =
            GenericValueStruct::from(GenericValueStructInit {
                gen_field1: "hey".into(),
                gen_field2: "hey2".into(),
            })
            .with_opt_gen_field1("hey".into());

        assert_eq!(g1.gen_field1, String::from("hey"));
        assert_eq!(g1.opt_gen_field1, Some(String::from("hey")));
    }

    #[test]
    fn generic_struct_with_bounds_test() {
        let g1: GenericValueStructWithBounds<i64> =
            GenericValueStructWithBounds::from(GenericValueStructWithBoundsInit { gen_field1: 17 })
                .with_opt_gen_field1(37);

        assert_eq!(g1.gen_field1, 17);
        assert_eq!(g1.opt_gen_field1, Some(37));
    }

    #[test]
    fn generic_struct_with_bounds_where_test() {
        let g1: GenericValueStructWithBoundsWhere<i64> =
            GenericValueStructWithBoundsWhere::from(GenericValueStructWithBoundsWhereInit {
                gen_field1: 17,
            })
            .with_opt_gen_field1(37);

        assert_eq!(g1.gen_field1, 17);
        assert_eq!(g1.opt_gen_field1, Some(37));
    }

    #[test]
    fn struct_with_defaults_test() {
        let sd1 = StructWithDefault::from(StructWithDefaultInit {
            req_field1: "test".into(),
        });

        assert_eq!(sd1.req_field2, 10);
        assert_eq!(sd1.opt_field2, Some(11));
    }

    #[test]
    fn opt_setter_struct() {
        let s1: SimpleStrValueStruct = SimpleStrValueStructInit {
            req_field1: "hey".into(),
            req_field2: 0,
        }
        .into();

        let s11 = s1.clone().opt_opt_field1(Some("hey".into()));

        assert_eq!(s11.opt_field1, Some(String::from("hey")));
    }

    #[test]
    fn different_access_struct() {
        let s1 = StructWithDifferentAccess::new("hey".into(), 0)
            .opt_field1("hey".into())
            .req_field2(0)
            .clone();

        assert_eq!(s1.opt_field1, Some("hey".into()));
    }

    #[test]
    fn struct_with_lifetimes() {
        let s1 = StructWithLifetime::new("hey").opt_field("hey").clone();

        assert_eq!(s1.opt_field, Some("hey"));
    }
}

#[cfg(test)]
mod field_names_tests {
    use rsb_derive::{Builder, BuilderFieldNames};

    #[derive(BuilderFieldNames)]
    #[allow(dead_code)]
    struct Plain {
        first: String,
        second: i32,
        third: Option<u8>,
    }

    #[derive(BuilderFieldNames)]
    #[allow(dead_code)]
    struct Generic<'a, T: Clone, const N: usize>
    where
        T: Default,
    {
        value: T,
        items: [T; N],
        label: &'a str,
    }

    #[derive(BuilderFieldNames)]
    #[allow(dead_code)]
    struct GenericSimple<T> {
        inner: T,
    }

    #[derive(BuilderFieldNames)]
    #[allow(dead_code)]
    struct RawIdent {
        r#type: String,
        r#match: i32,
        plain: bool,
    }

    #[derive(Debug, Clone, PartialEq, Builder, BuilderFieldNames)]
    struct BothDerives {
        req: String,
        #[default = "5"]
        with_default: i32,
        opt: Option<String>,
    }

    const PLAIN_FIELD_COUNT: usize = Plain::FIELD_NAMES.len();

    #[test]
    fn plain_struct_names_in_declaration_order() {
        assert_eq!(Plain::FIELD_NAMES, ["first", "second", "third"]);
    }

    #[test]
    fn generic_struct_names() {
        assert_eq!(GenericSimple::<i32>::FIELD_NAMES, ["inner"]);
        assert_eq!(
            Generic::<'static, i32, 3>::FIELD_NAMES,
            ["value", "items", "label"]
        );
    }

    #[test]
    fn raw_identifiers_lose_their_prefix() {
        assert_eq!(RawIdent::FIELD_NAMES, ["type", "match", "plain"]);
    }

    #[test]
    fn field_names_alongside_builder() {
        assert_eq!(BothDerives::FIELD_NAMES, ["req", "with_default", "opt"]);
        let built = BothDerives::new("r".into()).with_opt("o".into());
        assert_eq!(built.with_default, 5);
    }

    #[test]
    fn field_count_is_usable_in_const_context() {
        let sized: [u8; PLAIN_FIELD_COUNT] = [0; Plain::FIELD_NAMES.len()];
        assert_eq!(sized.len(), 3);
    }

    #[test]
    fn documented_struct_names() {
        assert_eq!(crate::documented::Header::FIELD_NAMES, ["name", "value"]);
    }
}

#[cfg(test)]
mod literal_default_tests {
    use rsb_derive::Builder;
    use smart_default::SmartDefault;

    #[derive(Debug, Clone, PartialEq, Builder)]
    struct LiteralDefaults {
        req: String,
        #[default = 100]
        int: i32,
        #[default = 12.5]
        float: f64,
        #[default = true]
        flag: bool,
        #[default = 'x']
        ch: char,
        #[default = b'a']
        byte: u8,
        #[default = "Some(11)"]
        opt_from_string: Option<i32>,
    }

    #[test]
    fn non_string_literal_is_the_default_value() {
        let s = LiteralDefaults::new("r".into());
        assert_eq!(s.int, 100);
        assert_eq!(s.float, 12.5);
        assert!(s.flag);
        assert_eq!(s.ch, 'x');
        assert_eq!(s.byte, b'a');
        assert_eq!(s.opt_from_string, Some(11));
    }

    #[test]
    fn literal_defaulted_fields_are_not_in_init() {
        let s: LiteralDefaults = LiteralDefaultsInit { req: "r".into() }.into();
        assert_eq!(s.int, 100);
        assert_eq!(s.with_int(1).int, 1);
    }

    /// SmartDefault reads the same `default` attribute: the name-value form
    /// is a default for both derives, the list form only for SmartDefault.
    #[derive(Debug, Clone, PartialEq, Builder, SmartDefault)]
    struct SharedWithSmartDefault {
        #[default = true]
        flag: bool,
        #[default = 100]
        count: i32,
        #[default(7)]
        listed: i32,
        name: Option<String>,
    }

    #[test]
    fn literal_defaults_coexist_with_smart_default() {
        let built = SharedWithSmartDefault::new(3);
        assert!(built.flag);
        assert_eq!(built.count, 100);
        assert_eq!(built.listed, 3);
        assert_eq!(built.name, None);

        let defaulted = SharedWithSmartDefault::default();
        assert!(defaulted.flag);
        assert_eq!(defaulted.count, 100);
        assert_eq!(defaulted.listed, 7);

        let from_init: SharedWithSmartDefault = SharedWithSmartDefaultInit { listed: 7 }.into();
        assert_eq!(from_init, defaulted);
    }
}
