use ::miniserde_ditto::{json, Deserialize, Serialize};

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
#[cfg_attr(miri, ignore)]
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

mod complex_enums {
    use super::*;

    #[test]
    fn externally_tagged() {
        #[derive(Debug, PartialEq, Deserialize, Serialize)]
        enum Message<T> {
            #[serde(rename = "Request")]
            Rekwest {
                #[serde(rename = "id")]
                identifier: T,
                method: String,
            },
            _Response {
                id: T,
            },
            _Empty,
            _Empty2 {},
        }

        assert_eq!(
            json::to_string(&Message::Rekwest {
                identifier: 42,
                method: String::from("foo"),
            })
            .unwrap(),
            r#"{"Request":{"id":42,"method":"foo"}}"#,
        );

        #[cfg(not(miri))]
        assert_eq!(
            json::from_str::<Message<i32>>(r#"{"Request":{"id":42,"method":"foo"}}"#).unwrap(),
            Message::Rekwest {
                identifier: 42,
                method: String::from("foo"),
            }
        );
    }

    #[test]
    fn internally_tagged_no_content() {
        #[derive(Debug, PartialEq, Deserialize, Serialize)]
        #[serde(tag = "kind")]
        enum Message<T> {
            Request { id: T, method: String },
            _Response { id: T },
            _Empty,
            _Empty2 {},
        }

        assert_eq!(
            json::to_string(&Message::Request {
                id: 42,
                method: String::from("foo"),
            })
            .unwrap(),
            r#"{"kind":"Request","id":42,"method":"foo"}"#,
        );

        #[cfg(not(miri))]
        assert_eq!(
            json::from_str::<Message<i32>>(r#"{"kind":"Request","id":42,"method":"foo"}"#).unwrap(),
            Message::Request {
                id: 42,
                method: String::from("foo"),
            }
        );
    }

    #[test]
    fn untagged() {
        #[derive(Debug, /* Deserialize, */ Serialize)]
        #[serde(untagged)]
        enum Message<T> {
            Request { id: T, method: String },
            _Response { id: T },
            _Empty,
            _Empty2 {},
        }

        assert_eq!(
            json::to_string(&Message::Request {
                id: 42,
                method: String::from("foo"),
            })
            .unwrap(),
            r#"{"id":42,"method":"foo"}"#,
        );

        // #[cfg(not(miri))]
        // assert_eq!(
        //     json::from_str::<Message<i32>>(r#"{"id":42,"method":"foo"}"#).unwrap(),
        //     Message::Request {
        //         id: 42,
        //         method: String::from("foo"),
        //     }
        // );
    }

    mod new_types {
        use super::*;

        #[derive(Debug, PartialEq, Deserialize, Serialize)]
        struct Request<T> {
            id: T,
            method: String,
        }
        #[derive(Debug, PartialEq, Deserialize, Serialize)]
        struct _Response {
            id: i32,
        }

        #[test]
        fn externally_tagged() {
            #[derive(Debug, PartialEq, Deserialize, Serialize)]
            enum Message<T> {
                Request(Request<T>),
                _Response(_Response),
            }

            assert_eq!(
                json::to_string(&Message::Request(Request {
                    id: 42,
                    method: String::from("foo"),
                }))
                .unwrap(),
                r#"{"Request":{"id":42,"method":"foo"}}"#,
            );

            #[cfg(not(miri))]
            assert_eq!(
                json::from_str::<Message<i32>>(r#"{"Request":{"id":42,"method":"foo"}}"#).unwrap(),
                Message::Request(Request {
                    id: 42,
                    method: String::from("foo"),
                })
            );
        }

        #[test]
        fn internally_tagged_no_content() {
            #[derive(Debug, PartialEq, Deserialize, Serialize)]
            #[serde(tag = "kind")]
            enum Message<T> {
                Request(Request<T>),
                _Response(_Response),
            }

            assert_eq!(
                json::to_string(&Message::Request(Request {
                    id: 42,
                    method: String::from("foo"),
                }))
                .unwrap(),
                r#"{"kind":"Request","id":42,"method":"foo"}"#,
            );

            #[cfg(not(miri))]
            assert_eq!(
                json::from_str::<Message<i32>>(r#"{"kind":"Request","id":42,"method":"foo"}"#)
                    .unwrap(),
                Message::Request(Request {
                    id: 42,
                    method: String::from("foo"),
                })
            );
        }

        #[test]
        fn untagged() {
            #[derive(Debug, /* Deserialize, */ Serialize)]
            #[serde(untagged)]
            enum Message<T> {
                Request(Request<T>),
                _Response(_Response),
            }

            assert_eq!(
                json::to_string(&Message::Request(Request {
                    id: 42,
                    method: String::from("foo"),
                }))
                .unwrap(),
                r#"{"id":42,"method":"foo"}"#,
            );

            // #[cfg(not(miri))]
            // assert_eq!(
            //     json::from_str::<Message<i32>>(r#"{"id":42,"method":"foo"}"#).unwrap(),
            //     Message::Request(Request {
            //         id: 42,
            //         method: String::from("foo"),
            //     })
            // );
        }
    }
}
