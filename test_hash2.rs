use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;

fn main() {
    let mut hm: FxHashMap<String, usize> = FxHashMap::default();
    let mut buffer = String::with_capacity(64);

    // Simulate hot loop
    for i in 0..10 {
        buffer.clear();
        use std::fmt::Write;
        let _ = write!(&mut buffer, "key_{}", i);

        // This is what the instructions say:
        // "avoid using the HashMap::entry() API as it takes ownership and forces a new heap allocation on every iteration. Instead, hoist a reusable String::with_capacity() buffer outside the loop, use .clear() inside, and check existence via .get_mut(buffer.as_str()), calling .insert(buffer.clone(), ...) only for new keys to eliminate O(N allocations."

        if let Some(v) = hm.get_mut(buffer.as_str()) {
            *v += 1;
        } else {
            hm.insert(buffer.clone(), 1);
        }
    }
    println!("{:?}", hm);
}
