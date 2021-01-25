use miniserde_ditto::{json, Deserialize, Serialize};

#[derive(PartialEq, Debug, Serialize, Deserialize)]
enum Tag {
    A,
    #[serde(rename = "renamedB")]
    B,
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
struct Example {
    x: String,
    t1: Tag,
    t2: Tag,
    n: Nested,
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
struct Nested {
    y: Option<Vec<String>>,
    z: Option<String>,
}

#[test]
fn test_de() {
    let j = r#" {"x": "X", "t1": "A", "t2": "renamedB", "n": {"y": ["Y", "Y"]}} "#;
    let actual: Example = json::from_str(j).unwrap();
    let expected = Example {
        x: "X".to_owned(),
        t1: Tag::A,
        t2: Tag::B,
        n: Nested {
            y: Some(vec!["Y".to_owned(), "Y".to_owned()]),
            z: None,
        },
    };
    assert_eq!(actual, expected);
}

#[test]
fn test_ser() {
    let example = Example {
        x: "X".to_owned(),
        t1: Tag::A,
        t2: Tag::B,
        n: Nested {
            y: Some(vec!["Y".to_owned(), "Y".to_owned()]),
            z: None,
        },
    };
    let actual = json::to_string(&example).unwrap();
    let expected = r#"{"x":"X","t1":"A","t2":"renamedB","n":{"y":["Y","Y"],"z":null}}"#;
    assert_eq!(actual, expected);
}

#[allow(dead_code)]
mod complex_enums {
    #[test]
    fn externally_tagged() {
        #[derive(Debug, ::miniserde_ditto::Serialize)]
        enum Message<T> {
            Request { id: T, method: String },
            Response { id: String },
        }
        assert_eq!(
            ::miniserde_ditto::json::to_string(&Message::Request {
                id: 42,
                method: String::from("foo"),
            })
            .unwrap(),
            r#"{"Request":{"id":42,"method":"foo"}}"#,
        );
    }

    #[test]
    fn internally_tagged_no_content() {
        #[derive(Debug, ::miniserde_ditto::Serialize)]
        #[serde(tag = "kind")]
        enum Message<T> {
            Request { id: T, method: String },
            Response { id: String },
        }
        assert_eq!(
            ::miniserde_ditto::json::to_string(&Message::Request {
                id: 42,
                method: String::from("foo"),
            })
            .unwrap(),
            r#"{"kind":"Request","id":42,"method":"foo"}"#,
        );
    }

    #[test]
    fn untagged() {
        #[derive(Debug, ::miniserde_ditto::Serialize)]
        #[serde(untagged)]
        enum Message<T> {
            Request { id: T, method: String },
            Response { id: String },
        }
        assert_eq!(
            ::miniserde_ditto::json::to_string(&Message::Request {
                id: 42,
                method: String::from("foo"),
            })
            .unwrap(),
            r#"{"id":42,"method":"foo"}"#,
        );
    }
}
