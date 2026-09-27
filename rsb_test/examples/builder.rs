//! Run with `cargo run -p rsb_test --example builder`.

use rsb_derive::{Builder, BuilderFieldNames};

#[derive(Debug, Clone, PartialEq, Builder, BuilderFieldNames)]
pub struct Connection {
    /// Host name or IP address to connect to.
    pub host: String,
    pub port: u16,
    pub user: Option<String>,
    #[default = "30"]
    pub timeout_secs: u64,
}

fn main() {
    // `new` takes only the required fields, in declaration order
    let local = Connection::new("localhost".into(), 5432);

    // The Init struct gives you named arguments for the required fields
    let remote: Connection = ConnectionInit {
        host: "db.example.com".into(),
        port: 5433,
    }
    .into();

    let with_user = remote.with_user("admin".into()).with_timeout_secs(5);
    let anonymous = with_user.clone().without_user();

    println!("new:          {local:?}");
    println!("with_user:    {with_user:?}");
    println!("without_user: {anonymous:?}");
    println!("field names:  {:?}", Connection::FIELD_NAMES);
}
