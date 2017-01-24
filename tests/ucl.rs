extern crate ucl;

use ucl::*;

#[test]
fn test_parse_file() {
    let cfg = parse_file("tests/test.ucl").unwrap();
    assert_eq!(cfg["version"].unwrap::<f64>(), 1.0);
    assert_eq!(cfg.get("this_should_not_appear"), None);
    assert_eq!(cfg["this_must_appear"].unwrap::<bool>(), true);

    assert_eq!(cfg["general"]["user"].unwrap::<String>(), "nobody".to_owned());
    assert_eq!(cfg["general"]["daemon"].unwrap::<bool>(), true);
    assert_eq!(cfg["general"]["fork"].unwrap::<i64>(), 4);

    assert_eq!(cfg["site"]["log_rotate"].unwrap::<f64>(), (7*24*60*60) as f64);
    assert_eq!(cfg["site"]["bind"][0].unwrap::<String>(), ":80".to_owned());
    assert_eq!(cfg["site"]["bind"][1].unwrap::<String>(), ":443".to_owned());

    assert_eq!(cfg["site"]["api"]["timeout"].unwrap::<f64>(), 1_f64);
    assert_eq!(cfg["site"]["api"]["max_recv_size"].unwrap::<i64>(), 25*1024*1024);
    assert_eq!(cfg["site"]["api"]["permissions"][0]["user"].unwrap::<String>(), "root".to_owned());
    assert_eq!(cfg["site"]["api"]["permissions"][0]["role"].unwrap::<String>(), "admin".to_owned());
    assert_eq!(cfg["site"]["api"]["permissions"][1]["user"].unwrap::<String>(), "guest".to_owned());
    assert_eq!(cfg["site"]["api"]["permissions"][1]["role"].unwrap::<String>(), "".to_owned());
    assert_eq!(cfg["site"]["api"].get("upstream"), None);

    assert_eq!(cfg["site"]["www"]["base_dir"].unwrap::<String>(), "/var/www".to_owned());
    assert_eq!(cfg["site"]["www"]["timeout"].unwrap::<f64>(), 0.1);
    assert_eq!(cfg["site"]["www"]["index"][0].unwrap::<String>(), "index.html".to_owned());
    assert_eq!(cfg["site"]["www"]["index"][1].unwrap::<String>(), "index.htm".to_owned());
    assert_eq!(cfg["site"]["www"]["file_types"][0].unwrap::<String>(), ".html".to_owned());
    assert_eq!(cfg["site"]["www"]["file_types"][1].unwrap::<String>(), ".css".to_owned());
    assert_eq!(cfg["site"]["www"]["file_types"][2].unwrap::<String>(), ".js".to_owned());
}
