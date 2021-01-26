use miniserde_ditto::json::{self, Value};

#[test]
#[cfg_attr(miri, ignore)]
fn test_round_trip_deeply_nested() {
    let mut j = String::new();
    for _ in 0..100_000 {
        j.push_str("{\"x\":[");
    }
    for _ in 0..100_000 {
        j.push_str("]}");
    }

    let value: Value = json::from_str(&j).unwrap();
    let j2 = json::to_string(&value).unwrap();
    assert_eq!(j, j2);
}
