use mem_profile::ProfilingAllocator;
use std::alloc::System;

#[global_allocator]
static ALLOCATOR: ProfilingAllocator<System> = ProfilingAllocator::new(System);

fn main() {
    let _guard = mem_profile::init();

    let mut data = Vec::new();
    for i in 0..10000 {
        data.push(format!("allocation-{}", i));
    }

    let _ = mem_profile::report::write_flamegraph("flamegraph.svg");
}
