use std::ops::Index;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::convert::{From, AsRef};
use std::io::{self, Read};
use std::fs::File;
use std::path::Path;

mod ucl {
    use std::cell::RefCell;

    thread_local!(static TERMINATOR: RefCell<Option<String>> = RefCell::new(None));

    include!(concat!(env!("OUT_DIR"), "/ucl.rs"));
}

pub use ucl::ParseError;

#[derive(Debug)]
pub enum UclError {
    Io(io::Error),
    Parse(ParseError)
}

impl From<io::Error> for UclError {
    fn from(err: io::Error) -> UclError {
        UclError::Io(err)
    }
}

impl From<ParseError> for UclError {
    fn from(err: ParseError) -> UclError {
        UclError::Parse(err)
    }
}

pub fn parse<T: AsRef<str> + ?Sized>(s: &T) -> Result<Value, ParseError> {
    ucl::ucl(s.as_ref())
}

pub fn parse_file<T: AsRef<Path> + ?Sized>(filename: &T) -> Result<Value, UclError> {
    let mut source = String::new();
    File::open(filename.as_ref())?.read_to_string(&mut source)?;
    Ok(parse(&source)?)
}

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
    pub fn unwrap<T: FromUcl>(&self) -> T {
        T::from_ucl(self).unwrap()
    }

    pub fn unwrap_or<T: FromUcl>(&self, def: T) -> T {
        T::from_ucl(self).unwrap_or(def)
    }

    pub fn get<T: AsRef<str>>(&self, key: T) -> Option<&Value> {
        match self {
            &Value::Object(ref v) => v.get(key.as_ref()),
            _ => None,
        }
    }

    pub fn get_or<T: FromUcl>(&self, key: &str, def: T) -> T {
        match self.get(key) {
            Some(v) => T::from_ucl(v).unwrap_or(def),
            None => def,
        }
    }

    fn merge(&mut self, other: Value) {
        if let Value::Object(other) = other {
            if let Value::Object(ref mut m) = *self {
                for (k, v) in other {
                    match m.entry(k) {
                        Entry::Occupied(mut o) => o.get_mut().merge(v),
                        Entry::Vacant(o) => { o.insert(v); },
                    }
                }
            } else {
                *self = Value::Object(other);
            }
        } else {
            *self = other;
        }
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
        self.get(key).expect("no entry found for key")
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

impl From<Vec<(String, Value)>> for Value {
    fn from(kvs: Vec<(String, Value)>) -> Self {
        let kvs = kvs.into_iter()
            .map(|(k, v)| (Key::from(k), v))
            .collect();
        Value::Object(construct_object(kvs))
    }
}

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
    let mut rv: HashMap<String, Value> = HashMap::new();
    for (k, v) in m {
        match k {
            Key::Single(key) => {
                match rv.entry(key) {
                    Entry::Occupied(mut o) => o.get_mut().merge(v),
                    Entry::Vacant(o) => { o.insert(v); },
                }
            },
            Key::Multiple(keys) => {
                let mut iter = keys.into_iter().rev();
                let mut key = iter.next().unwrap();
                let mut value = v;
                for k in iter {
                    value = Value::Object([(key, value)].iter().cloned().collect());
                    key = k;
                }

                match rv.entry(key) {
                    Entry::Occupied(mut o) => o.get_mut().merge(value),
                    Entry::Vacant(o) => { o.insert(value); },
                }
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
    //     assert_eq!(m, expect);
    // }

    #[test]
    fn test_json() {
        assert_eq!(ucl::jsonValue("100").unwrap(), Value::from(100));
        assert_eq!(ucl::jsonValue("1.23").unwrap(), Value::from(1.23));
        assert_eq!(ucl::jsonValue("true").unwrap(), Value::from(true));
        assert_eq!(ucl::jsonValue("false").unwrap(), Value::from(false));
        assert_eq!(ucl::jsonValue("null").unwrap(), Value::Null);
        assert_eq!(ucl::jsonValue(r#""string""#).unwrap(), Value::from("string"));

        let v = ucl::jsonArray(r#"[1, "two", true]"#).unwrap();
        assert_eq!(v, Value::from(vec![
            Value::from(1),
            Value::from("two"),
            Value::from(true)
        ]));

        let v = ucl::jsonValue(r#"{
          "param": "value",
          "param1": "value1",
          "param": "value2"
        }"#).unwrap();
        assert_eq!(v, Value::from(vec![
            (Key::from("param"), Value::from(vec![
                Value::from("value"),
                Value::from("value2")
            ])),
            (Key::from("param1"), Value::from("value1"))
        ]));
    }

    #[test]
    fn test_ucl_array() {
        assert_eq!(ucl::array("[]").unwrap(), Value::Array(vec![]));
        assert_eq!(ucl::array(r#"[1, "two", true]"#).unwrap(), Value::from(vec![
            Value::from(1),
            Value::from("two"),
            Value::from(true)
        ]));
        assert_eq!(ucl::array(r#"[
          100,
          [],
          [true,false]
        ]"#).unwrap(), Value::from(vec![
            Value::from(100),
            Value::Array(vec![]),
            Value::from(vec![Value::from(true), Value::from(false)])
        ]));
        assert_eq!(ucl::array("[.foo, .bar]").unwrap(), Value::from(vec![
            Value::from(".foo"),
            Value::from(".bar")
        ]));
        assert_eq!(ucl::array("[{},{}]").unwrap(), Value::from(vec![
            Value::Object(HashMap::new()),
            Value::Object(HashMap::new())]
        ));
        assert_eq!(ucl::keyValue(r#"object_array = [{
          index = 1;
          value = foo;
        }, {
          index = 2;
          value = bar;
        }];"#).unwrap(), (Key::from("object_array"), Value::from(vec![
            Value::from(vec![
                (Key::from("index"), Value::from(1)),
                (Key::from("value"), Value::from("foo"))
            ]),
            Value::from(vec![
                (Key::from("index"), Value::from(2)),
                (Key::from("value"), Value::from("bar"))
            ])
        ])));
    }

    #[test]
    fn test_ucl_object() {
        assert_eq!(parse(r#"obj a {
          name = A;
        }

        obj b {
          name = B;
        }"#).unwrap(), Value::from(vec![
            (Key::from("obj"), Value::from(vec![
                (Key::from("a"), Value::from(vec![
                    (Key::from("name"), Value::from("A"))
                ])),
                (Key::from("b"), Value::from(vec![
                    (Key::from("name"), Value::from("B"))
                ]))
            ]))
        ]));
    }

    #[test]
    fn test_ucl_value() {
        assert_eq!(ucl::value("10").unwrap(), Value::from(10));
        assert_eq!(ucl::value("-10").unwrap(), Value::from(-10));
        assert_eq!(ucl::value("0x1f").unwrap(), Value::from(31));
        assert_eq!(ucl::value("0xFE").unwrap(), Value::from(254));

        assert_eq!(ucl::value("1.23").unwrap(), Value::from(1.23_f64));
        assert_eq!(ucl::value("-1.23").unwrap(), Value::from(-1.23_f64));

        assert_eq!(ucl::value("1k").unwrap(), Value::from(1000));
        assert_eq!(ucl::value("1kb").unwrap(), Value::from(1024));
        assert_eq!(ucl::value("10ms").unwrap(), Value::from(0.01));
        assert_eq!(ucl::value("1.2min").unwrap(), Value::from(72_f64));

        assert_eq!(ucl::value("true").unwrap(), Value::from(true));
        assert_eq!(ucl::value("false").unwrap(), Value::from(false));

        assert_eq!(ucl::value("null").unwrap(), Value::Null);

        assert_eq!(ucl::value("foo").unwrap(), Value::from("foo"));
        assert_eq!(ucl::value(r#""foo""#).unwrap(), Value::from("foo"));
    }

    #[test]
    fn it_works() {
        assert_eq!(ucl::key("foo").unwrap(), "foo".to_owned());
        assert_eq!(ucl::key(r#""foo""#).unwrap(), "foo".to_owned());

        assert_eq!(ucl::keyValue("param = value;").unwrap(), (Key::from("param"), Value::from("value")));
        assert_eq!(ucl::keyValue(r#"section {
          param1=value1;
          param2=value2;
        }"#).unwrap(), (Key::from("section"), Value::from(vec![
            (Key::from("param1"), Value::from("value1")),
            (Key::from("param2"), Value::from("value2"))
        ])));

        ucl::keyValue("foo bar = { foo = bar; }").unwrap();

        assert_eq!(ucl::object("{}").unwrap(), Value::from(HashMap::new()));
        assert_eq!(ucl::object(r#"{
          param1=value1;
          param2=value2;
        }"#).unwrap(), Value::from(vec![
            (Key::from("param1"), Value::from("value1")),
            (Key::from("param2"), Value::from("value2"))
        ]));

    }

    #[test]
    fn test_ucl_multiline_string() {
        assert_eq!(ucl::multiLineString(r#"<<EOD
EOD
"#).unwrap(), "");

        assert_eq!(ucl::multiLineString(r#"<<EOD
foo
EOF
EOD
"#).unwrap(), "foo\nEOF");
        assert_eq!(ucl::multiLineString(r#"<<EOS

some
text

EOS
"#).unwrap(), "\nsome\ntext\n");
        assert_eq!(parse(r#"s1 = <<EOS
some
text
EOS
;
                            s2 = <<EOD
EOD
;
"#).unwrap(), Value::from(vec![
    (Key::from("s1"), Value::from("some\ntext")),
    (Key::from("s2"), Value::from(""))
]));
    }

    #[test]
    fn test_complex() {
        let v = parse(r#"param = value;
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

        assert_eq!(v, Value::from(vec![
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

        assert_eq!(v["param"].unwrap::<String>(), "value");
        assert_eq!(v["section"]["flag"].unwrap::<bool>(), true);
        assert_eq!(v["section"]["subsection"]["host"][1]["port"].unwrap::<i64>(), 901);
        let hosts: Array = v["section"]["subsection"]["host"].unwrap();
        for i in 0..hosts.len() {
            let port: i64 = hosts[i]["port"].unwrap();
            assert_eq!(port, 900 + i as i64);
        }

        assert_eq!(v.get("non_exist"), None);
        assert_eq!(v.get_or("non_exist", 0), 0);
    }
}
