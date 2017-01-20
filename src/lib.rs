use std::ops::Index;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Array(Array),
    Object(Object),
    Null,
}

pub type Array = Vec<Value>;
pub type Object = HashMap<String, Value>;

impl Value {
    fn unwrap<T: FromUcl>(&self) -> T {
        T::from_ucl(self).unwrap()
    }
}

// impl Index<String> for Value {
//     type Output = Value;

//     fn index(&self, key: String) -> &Self::Output {
//         match self {
//             &Value::Object(ref m) => {
//                 m.get(&key).unwrap()
//             },
//             _ => panic!()
//         }
//     }
// }

impl<'a> Index<&'a str> for Value {
    type Output = Value;

    fn index(&self, key: &'a str) -> &Self::Output {
        match self {
            &Value::Object(ref v) => v.get(key).unwrap(),
            _ => panic!()
        }
    }
}

impl Index<usize> for Value {
    type Output = Value;

    fn index(&self, idx: usize) -> &Self::Output {
        match self {
            &Value::Array(ref v) => &v[idx],
            _ => panic!()
        }
    }
}

impl<'a> From<&'a str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_owned())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self{
        Value::String(s)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(n)
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Float(n)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Boolean(b)
    }
}

impl From<Array> for Value {
    fn from(items: Array) -> Self {
        Value::Array(items)
    }
}

impl From<Object> for Value {
    fn from(obj: Object) -> Self {
        Value::Object(obj)
    }
}

// impl From<Vec<(String, Value)>> for Value {
//     fn from(kvs: Vec<(String, Value)>) -> Self {
//         Value::Object(object_from_kvs(kvs))
//     }
// }

// fn object_from_kvs(kvs: Vec<(String, Value)>) -> Object {
//     let mut m: HashMap<String, Value> = HashMap::new();
//     for (k, v) in kvs {
//         if !m.contains_key(&k) {
//             m.insert(k, v);
//         } else {
//             let newval = match m.remove(&k).unwrap() {
//                 Value::Array(mut items) => {
//                     items.push(v);
//                     Value::Array(items)
//                 },
//                 Value::Null => v,
//                 other @ _ => Value::Array(vec![other, v]),
//             };
//             m.insert(k, newval);
//         }
//     }
//     m
// }

impl From<Vec<(Key, Value)>> for Value {
    fn from(kvs: Vec<(Key, Value)>) -> Self {
        Value::Object(construct_object(kvs))
    }
}

fn construct_object(kvs: Vec<(Key, Value)>) -> Object {
    // Phase 1: non-unique items into an array
    let mut m = HashMap::new();
    for (k, v) in kvs {
        if !m.contains_key(&k) {
            m.insert(k, v);
        } else {
            let newval = match m.remove(&k).unwrap() {
                Value::Array(mut items) => {
                    items.push(v);
                    Value::Array(items)
                },
                Value::Null => v,
                other @ _ => Value::Array(vec![other, v]),
            };
            m.insert(k, newval);
        }
    }

    // Phase 2: multiple keys into multi-dimensional HashMap
    let mut rv = HashMap::new();
    for (k, v) in m {
        match k {
            Key::Single(key) => {
                rv.insert(key, v);
            },
            Key::Multiple(keys) => {
                let mut iter = keys.into_iter().rev();
                let mut key = iter.next().unwrap();
                let mut value = v;
                for k in iter {
                    value = Value::Object([(key, value)].iter().cloned().collect());
                    key = k;
                }
                rv.insert(key, value);
            },
        }
    }
    rv
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Single(String),
    Multiple(Vec<String>),
}

impl<'a> From<&'a str> for Key {
    fn from(s: &'a str) -> Self {
        Key::Single(s.to_owned())
    }
}

impl From<String> for Key {
    fn from(s: String) -> Self {
        Key::Single(s)
    }
}

impl From<Vec<String>> for Key {
    fn from(mut keys: Vec<String>) -> Self {
        if keys.len() == 1 {
            Key::Single(keys.swap_remove(0))
        } else {
            Key::Multiple(keys)
        }
    }
}

pub trait FromUcl where Self: Sized {
    fn from_ucl(v: &Value) -> Option<Self>;
}

impl FromUcl for i64 {
    fn from_ucl(v: &Value) -> Option<Self> {
        match v {
            &Value::Number(n) => Some(n),
            _ => None,
        }
    }
}

impl FromUcl for f64 {
    fn from_ucl(v: &Value) -> Option<Self> {
        match v {
            &Value::Float(n) => Some(n),
            _ => None,
        }
    }
}

impl FromUcl for String {
    fn from_ucl(v: &Value) -> Option<Self> {
        match v {
            &Value::String(ref s) => Some(s.clone()),
            _ => None,
        }
    }
}

impl FromUcl for bool {
    fn from_ucl(v: &Value) -> Option<Self> {
        match v {
            &Value::Boolean(b) => Some(b),
            _ => None,
        }
    }
}

impl FromUcl for Array {
    fn from_ucl(v: &Value) -> Option<Self> {
        match v {
            &Value::Array(ref x) => Some(x.clone()),
            _ => None,
        }
    }
}

impl FromUcl for Object {
    fn from_ucl(v: &Value) -> Option<Self> {
        match v {
            &Value::Object(ref x) => Some(x.clone()),
            _ => None,
        }
    }
}

mod ucl {
    include!(concat!(env!("OUT_DIR"), "/ucl.rs"));
}

#[cfg(test)]
mod tests {
    use super::ucl;
    use super::*;
    use std::collections::HashMap;

    // #[test]
    // fn test_hash_map() {
    //     let kvs = vec![
    //         ("param".to_owned(), Value::from("value")),
    //         ("param".to_owned(), Value::from("value2")),
    //         ("param1".to_owned(), Value::from("value1"))
    //     ];
    //     let m = object_from_kvs(kvs);
    //     let expect: HashMap<String, Value> = [
    //         ("param".to_owned(), Value::Array(vec![
    //             Value::from("value"),
    //             Value::from("value2"),
    //         ])),
    //         ("param1".to_owned(), Value::from("value1"))
    //     ].iter().cloned().collect();
    //     assert!(m == expect);
    // }

    #[test]
    fn it_works() {
        assert!(ucl::key("foo").unwrap() == "foo".to_owned());
        assert!(ucl::key(r#""foo""#).unwrap() == "foo".to_owned());

        assert!(ucl::value("foo").unwrap() == Value::from("foo"));
        assert!(ucl::value(r#""foo""#).unwrap() == Value::from("foo"));
        assert!(ucl::value("100").unwrap() == Value::from(100));
        assert!(ucl::value("1k").unwrap() == Value::from(1000));
        assert!(ucl::value("1kb").unwrap() == Value::from(1024));
        assert!(ucl::value("10ms").unwrap() == Value::from(0.01));
        assert!(ucl::value("true").unwrap() == Value::from(true));
        assert!(ucl::value("false").unwrap() == Value::from(false));
        assert!(ucl::value("null").unwrap() == Value::Null);

        assert!(ucl::keyvalue("param = value;").unwrap() == (Key::from("param"), Value::from("value")));
        assert!(ucl::keyvalue(r#"section {
          param1=value1;
          param2=value2;
        }"#).unwrap() == (Key::from("section"), Value::from(vec![
            (Key::from("param1"), Value::from("value1")),
            (Key::from("param2"), Value::from("value2"))
        ])));

        ucl::keyobject("foo bar = { foo = bar; }").unwrap();

        assert!(ucl::object("{}").unwrap() == Value::from(HashMap::new()));
        assert!(ucl::object(r#"{
          param1=value1;
          param2=value2;
        }"#).unwrap() == Value::from(vec![
            (Key::from("param1"), Value::from("value1")),
            (Key::from("param2"), Value::from("value2"))
        ]));

        let complex = ucl::keyvalues(r#"param = value;
          section {
            param = value;
            param1 = value1;
            flag = true;
            number = 10k;
            time = 0.2s;
            string = "something";
            subsection {
              host = {
                host = "hostname";
                port = 900;
              }
              host = {
                host = "hostname";
                port = 901;
              }
            }
          }
        "#).unwrap();

        assert!(complex == Value::from(vec![
            (Key::from("param"), Value::from("value")),
            (Key::from("section"), Value::from(vec![
                (Key::from("param"), Value::from("value")),
                (Key::from("param1"), Value::from("value1")),
                (Key::from("flag"), Value::from(true)),
                (Key::from("number"), Value::from(10_000)),
                (Key::from("time"), Value::from(0.2)),
                (Key::from("string"), Value::from("something")),
                (Key::from("subsection"), Value::from(vec![
                    (Key::from("host"), Value::from(vec![
                        Value::from(vec![
                            (Key::from("host"), Value::from("hostname")),
                            (Key::from("port"), Value::from(900))
                        ]),
                        Value::from(vec![
                            (Key::from("host"), Value::from("hostname")),
                            (Key::from("port"), Value::from(901))
                        ]),
                    ]))
                ]))
            ]))
        ]));

        assert!(complex["param"].unwrap::<String>() == "value");
        assert!(complex["section"]["flag"].unwrap::<bool>() == true);
        assert!(complex["section"]["subsection"]["host"][1]["port"].unwrap::<i64>() == 901);
        let hosts: Array = complex["section"]["subsection"]["host"].unwrap();
        for i in 0..hosts.len() {
            let port: i64 = hosts[i]["port"].unwrap();
            assert!(port == 900 + i as i64);
        }
    }
}
