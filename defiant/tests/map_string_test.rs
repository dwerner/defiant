//! Test map field with string values

use defiant_derive::View;
use defiant::{Arena, ArenaMap, Encode};

#[derive(defiant_derive::View)]
struct MapTest<'arena> {
    #[defiant(btree_map = "string, string", tag = "1")]
    map_field: ArenaMap<'arena, &'arena str, &'arena str>,
}

#[test]
fn test_map_encode() {
    let arena = Arena::new();
    let msg = MapTest {
        map_field: ArenaMap::default(),
    };
    let mut buf = Vec::new();
    msg.encode(&mut buf).unwrap();
}
