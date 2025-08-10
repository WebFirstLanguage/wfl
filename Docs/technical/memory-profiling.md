# Memory Profiling with dhat-rs in WFL

This comprehensive guide explains how to use dhat-rs memory profiling in the WebFirst Language (WFL) project.

## Overview

WFL integrates with [dhat-rs](https://docs.rs/dhat/), a Rust heap profiling library, to provide:

1. **Heap profiling**: Track all allocations with the `dhat-heap` feature
2. **Ad-hoc profiling**: Instrument specific hotspots with the `dhat-ad-hoc` feature
3. **Heap usage tests**: Verify memory usage in CI

## Setup and Configuration

### Project Configuration

The WFL project is already configured with dhat-rs as an optional dependency in `Cargo.toml`:

```toml
[dependencies]
dhat = { version = "0.3.0", optional = true }

[features]
dhat-heap = ["dhat"]    # for heap profiling
dhat-ad-hoc = ["dhat"]  # for ad-hoc profiling

[profile.release]
debug = 1  # Enable backtraces without significant performance impact
```

### Global Allocator

The global allocator is defined in `src/lib.rs`:

```rust
#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;
```

### Profiler Initialization

Profilers are initialized in both `src/main.rs` and `src/repl.rs`:

```rust
// Initialize dhat profiler if enabled
#[cfg(feature = "dhat-heap")]
let _profiler = dhat::Profiler::new_heap();

#[cfg(feature = "dhat-ad-hoc")]
let _profiler = dhat::Profiler::new_ad_hoc();
```

## Running Profiling

### Heap Profiling

Heap profiling tracks all memory allocations and deallocations in your program:

```bash
cargo run --features dhat-heap --release
```

This will generate a `dhat-heap.json` file in the project root that contains detailed information about all heap allocations.

#### Example: Profiling a Specific WFL Script

```bash
cargo run --features dhat-heap --release -- path/to/your/script.wfl
```

### Ad-hoc Profiling 

Ad-hoc profiling allows you to instrument specific code locations:

```bash
cargo run --features dhat-ad-hoc --release
```

This will generate a `dhat-ad-hoc.json` file in the project root that contains information about the instrumented hotspots.

#### Example: Profiling the REPL

```bash
cargo run --features dhat-ad-hoc --release
```

Then use the REPL normally, and the ad-hoc events will be recorded.

## Viewing and Interpreting Results

### Using the Online Viewer

The JSON files can be viewed using the [online dhat viewer](https://nnethercote.github.io/dh_view.html):

1. Open the viewer in your web browser
2. Load the generated JSON file
3. Explore the allocation tree and hotspots

### Understanding Heap Profiling Results

The heap profiling results show:

- **Total allocations**: Number of memory allocations made
- **Total bytes**: Total memory allocated
- **Peak bytes**: Maximum memory usage at any point
- **Call tree**: Hierarchical view of where allocations occurred

#### Key Metrics to Watch

- **Peak memory usage**: Should stay below your target threshold
- **Allocation count**: High counts may indicate inefficient memory usage
- **Allocation hotspots**: Functions that allocate the most memory

### Understanding Ad-hoc Profiling Results

Ad-hoc profiling results show:

- **Event counts**: How many times each instrumented location was hit
- **Call tree**: Where the ad-hoc events were triggered from

## Adding Custom Instrumentation

### Instrumenting New Hotspots

To add ad-hoc instrumentation to a new hotspot:

```rust
#[cfg(feature = "dhat-ad-hoc")]
dhat::ad_hoc_event(1);  // Weight = 1 is sufficient for most cases
```

### Weighted Events

You can assign different weights to events to highlight their importance:

```rust
#[cfg(feature = "dhat-ad-hoc")]
dhat::ad_hoc_event(5);  // Higher weight for more significant operations
```

### Instrumenting Memory-Intensive Operations

Good candidates for instrumentation include:

- Functions that create or manipulate large data structures
- Operations that might cause reference cycles
- Recursive functions that could cause stack overflows
- Functions called in tight loops

## Running Memory Tests

Memory tests verify that memory usage remains within expected limits:

```bash
cargo test --features dhat-heap --release -- --test-threads 1 memory_usage
```

The single-threaded execution ensures consistent memory measurements.

### Writing Your Own Memory Tests

To create a new memory test:

```rust
#[test]
fn my_memory_test() {
    let _profiler = dhat::Profiler::builder().testing().build();
    
    // Perform operations that allocate memory
    let result = perform_memory_intensive_operation();
    
    // Check memory usage
    let stats = dhat::HeapStats::get();
    assert!(stats.max_bytes < 50 * 1024, 
            "Memory usage too high: {} bytes", stats.max_bytes);
    
    // Verify operation result
    assert!(result.is_ok());
}
```

### Running Tests in CI

The CI workflow includes Windows-specific memory tests:

```yaml
- name: Memory-usage tests
  run: cargo test --features dhat-heap --release -- --test-threads 1 memory_usage
```

## Implementation Details

### Instrumented Hotspots

Ad-hoc instrumentation points are placed in key memory hotspots:

- `Interpreter::call_function` - Function call operations
- `Environment::new_global` - Global environment creation
- `Environment::new` - Environment creation with parent
- `Environment::new_child_env` - Child environment creation

### Memory Usage Tests

Memory usage tests in `tests/memory_usage.rs` include:

- `basic_allocations`: Tests simple vector allocation
- `interpreter_small_program`: Tests memory usage of a small WFL program
- `test_functions_memory_usage`: Tests memory usage of WFL functions
- `test_environment_memory_usage`: Tests memory usage of WFL environments

## Platform-Specific Notes

### Windows

- Windows debugging information may cause slower performance
- Set `debug = 1` in the release profile (already configured in Cargo.toml)
- Always run with `--release` flag for reasonable performance
- CI includes Windows-specific memory tests to ensure cross-platform compatibility

### Linux

- Performance impact is generally lower on Linux
- Same profiling commands work without modification
- Consider using `perf` alongside dhat for more comprehensive profiling

## Best Practices

### Memory Optimization Strategies

Based on profiling results, consider these optimization strategies:

1. **Reduce allocations in hot loops**:
   - Reuse buffers instead of creating new ones
   - Use stack allocation for small, fixed-size data

2. **Break reference cycles**:
   - Use `Weak` references for parent-child relationships
   - Implement proper drop patterns for cyclic data structures

3. **Optimize string handling**:
   - Use string interning for repeated strings
   - Avoid unnecessary string concatenations

4. **Reduce clone operations**:
   - Use references where possible
   - Implement Copy for small types

### Profiling Workflow

For effective memory profiling:

1. Run baseline profiling to establish normal memory usage
2. Identify hotspots and unexpected memory usage
3. Make targeted optimizations
4. Re-run profiling to verify improvements
5. Add memory tests to prevent regressions

## Troubleshooting

### Performance Issues

If the profiler seems to be running too slowly:
- Always use `--release` mode
- Ensure you're not capturing too many ad-hoc events
- For large programs, consider using targeted profiling of specific sections
- Limit profiling to specific parts of your code by conditionally initializing the profiler

### Output Problems

If the profiler isn't generating output:
- Ensure the `_profiler` binding is kept alive for the entire program
- Check that you're using the correct feature flag
- Verify file permissions in the output directory
- Make sure you're not calling `std::process::exit()` without dropping the profiler first

### Memory Usage Issues

If you encounter unexpected memory usage:
- Check for reference cycles between Environment and FunctionValue
- Verify that weak references are used appropriately
- Run memory tests with `RUST_BACKTRACE=1` for more detailed allocation information
- Use `Rc::strong_count` to verify reference counts are as expected
- After dropping objects that might participate in reference cycles, assert that reference counts reach expected values

### Common Errors

- **"Profiler already running"**: You've tried to initialize multiple profilers
- **Missing JSON output**: The profiler wasn't properly initialized or was dropped too early
- **Extremely slow execution**: You're running in debug mode; switch to `--release`
- **Inconsistent test results**: Tests are running in parallel; use `--test-threads 1`

## Integration with Other Memory Tools

### Combining with Heaptrack

For more comprehensive memory analysis, you can combine dhat-rs with Heaptrack:

```bash
# Build with debug symbols in release mode
cargo build --release

# Run under Heaptrack
heaptrack ./target/release/wfl --features dhat-heap
```

This provides both the detailed allocation tracking of dhat-rs and the system-level memory analysis of Heaptrack.

### Memory Leak Detection

To detect memory leaks, particularly reference cycles:

1. Run with dhat-heap enabled
2. Check the "total_blocks" value in the JSON output
3. Verify that all blocks are freed by the end of execution
4. Use the memory tests to assert on reference counts

## Practical Examples

### Example 1: Identifying Memory Leaks in Closures

WFL's function values can capture their environment, which might lead to reference cycles:

```rust
// WFL code that might cause a memory leak
define action create_counter():
    store count as 0
    
    define action increment():
        store count as count plus 1
        return count
    end action
    
    return increment
end action

store counter as create_counter()
```

Using dhat-rs, you can:
1. Run with `--features dhat-heap`
2. Look for `FunctionValue` allocations that aren't freed
3. Check the call stack to identify where the cycle occurs

### Example 2: Optimizing Parser Memory Usage

If profiling shows excessive allocations in the parser:

```rust
// Before optimization
let tokens = tokens.clone();
let mut parser = Parser::new(&tokens);

// After optimization (using references)
let mut parser = Parser::new(&tokens);
```

### Example 3: Monitoring Memory Usage in CI

The CI workflow automatically runs memory tests to catch regressions:

```yaml
- name: Memory-usage tests
  run: cargo test --features dhat-heap --release -- --test-threads 1 memory_usage
```

This ensures that memory usage doesn't unexpectedly increase between commits.
