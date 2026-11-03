// Upstream's regression test for the remove_found hang (8a4f6d91), run on the baseline.
use heapless::index_map::FnvIndexMap;
fn main() {
    let mut map: FnvIndexMap<u16, u16, 4> = FnvIndexMap::new();
    map.insert(0, 0).unwrap();
    map.insert(4, 4).unwrap();
    map.insert(8, 8).unwrap();
    println!("remove(&0) = {:?}", map.remove(&0));
}
