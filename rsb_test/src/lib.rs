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

    #[test]
    fn new_str_value_struct() {
        let s1: SimpleStrValueStruct = SimpleStrValueStruct::new("hey".into(), 0);

        assert_eq!(s1.req_field1, String::from("hey"));
    }

    #[derive(Debug, Clone, PartialEq, Builder)]
    struct StructWithDifferentAccess {
        pub req_field1: String,
        req_field2: i32,
        pub opt_field1: Option<String>,
        opt_field2: Option<i32>,
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
}
