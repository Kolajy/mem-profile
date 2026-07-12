use mem_profile::ProfilingAllocator;
use std::alloc::System;

#[global_allocator]
static ALLOC: ProfilingAllocator<System> = ProfilingAllocator::new(System);

#[test]
fn test_allocator_stats_tracking() {
    let initial_allocs = ALLOC.allocation_count();
    let initial_deallocs = ALLOC.deallocation_count();
    let initial_bytes = ALLOC.active_bytes();

    // Allocate a vector of 1024 bytes
    let data: Vec<u8> = vec![42; 1024];

    // Statistics should reflect the new allocation
    let after_alloc_count = ALLOC.allocation_count();
    let after_alloc_bytes = ALLOC.active_bytes();

    assert!(
        after_alloc_count > initial_allocs,
        "Expected allocation count to increase. Before: {}, After: {}",
        initial_allocs,
        after_alloc_count
    );
    assert!(
        after_alloc_bytes >= initial_bytes + 1024,
        "Expected active bytes to increase by at least 1024. Before: {}, After: {}",
        initial_bytes,
        after_alloc_bytes
    );

    // Let's print out the values to console during test execution
    println!(
        "Initial: Allocs={}, Bytes={}",
        initial_allocs, initial_bytes
    );
    println!(
        "After Alloc: Allocs={}, Bytes={}",
        after_alloc_count, after_alloc_bytes
    );

    // Deallocate the vector
    drop(data);

    let after_dealloc_count = ALLOC.deallocation_count();
    let after_dealloc_bytes = ALLOC.active_bytes();

    assert!(
        after_dealloc_count > initial_deallocs,
        "Expected deallocation count to increase. Before: {}, After: {}",
        initial_deallocs,
        after_dealloc_count
    );
    assert!(
        after_dealloc_bytes < after_alloc_bytes,
        "Expected active bytes to decrease after drop. Before: {}, After: {}",
        after_alloc_bytes,
        after_dealloc_bytes
    );

    println!(
        "After Dealloc: Deallocs={}, Bytes={}",
        after_dealloc_count, after_dealloc_bytes
    );
}
