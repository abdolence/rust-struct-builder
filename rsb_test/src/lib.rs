#[cfg(test)]
mod tests {

    use rsb_derive::Builder;

    #[derive(Debug, Clone,PartialEq, Builder)]
    struct SimpleStrValueStruct {
        req_field1: String,
        req_field2: i32,
        opt_field1: Option<String>,
        opt_field2: Option<i32>
    }

    #[test]
    fn fill_str_value_struct() {
        let s1 = SimpleStrValueStruct {
            req_field1 : "hey".into(),
            req_field2 : 0,
            opt_field1 : None,
            opt_field2 : None
        }.opt_field1("hey".into()).clone();

        assert_eq!(s1.opt_field1,Some("hey".into()));

        let s1c = SimpleStrValueStruct {
            req_field1 : "hey".into(),
            req_field2 : 0,
            opt_field1 : None,
            opt_field2 : None
        }.with_opt_field1("hey2".into());
        assert_eq!(s1c.opt_field1,Some("hey2".into()));

        assert_eq!(s1c.without_opt_field1().opt_field1,None);
    }

    #[test]
    fn new_str_value_struct() {
        let s1 : SimpleStrValueStruct = SimpleStrValueStruct::new(
            "hey".into(),
            0
        );

        assert_eq!(s1.req_field1,String::from("hey"));
    }

    #[test]
    fn into_str_value_struct() {
        let s1 : SimpleStrValueStruct =
            SimpleStrValueStructInit {
                req_field1 : "hey".into(),
                req_field2 : 0
            }.into();

        let s11 =
            s1.clone()
            .with_opt_field1("hey".into())
            .with_req_field2(10);

        assert_eq!(s1.req_field1,String::from("hey"));
    }

    #[test]
    fn all_together_test() {
        let s1 : SimpleStrValueStruct =
            SimpleStrValueStruct::from(
                SimpleStrValueStructInit {
                    req_field1 : "hey".into(),
                    req_field2 : 0
                }
            )
                .with_opt_field1("hey".into())
                .with_opt_field2(10);

        assert_eq!(s1.req_field1,String::from("hey"));
        assert_eq!(s1.opt_field1,Some(String::from("hey")));
    }

}
