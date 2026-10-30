//! Contract conformance: descriptor is serializable/round-trippable.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use zk_contract_package::{Package, PackageDescriptor};

struct Dummy;

impl Package for Dummy {
    fn descriptor(&self) -> PackageDescriptor {
        PackageDescriptor {
            id: "zeokeel/dummy".to_owned(),
            version: "0.1.0".to_owned(),
            kind: "capability".to_owned(),
            roles: vec!["tool.dummy".to_owned()],
        }
    }
}

#[test]
fn descriptor_round_trips() {
    let d = Dummy.descriptor();
    let json = serde_json::to_string(&d).unwrap();
    let back: PackageDescriptor = serde_json::from_str(&json).unwrap();
    assert_eq!(d, back);
}
