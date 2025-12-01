# WFL Memory Optimization Guidelines

This document contains guidelines and best practices for memory optimization in the WFL interpreter, based on lessons learned from memory leak investigations and performance profiling.

## Memory Leak Prevention

### 1. Break Reference Cycles

To properly fix memory leaks in the WFL codebase:

**Convert strong references to weak references for back-references:**
- Change `FunctionValue::env` from `Rc<RefCell<Environment>>` to `Weak<RefCell<Environment>>`
- Update all constructors to use `Rc::downgrade(&env)` instead of `Rc::clone(&env)`
- Audit for other cycles in object methods and callbacks

### 2. Optimize Parser Memory Usage

**Reduce allocations during parsing:**
- Replace `.cloned()` token peeks with references
- Use `reserve()` for vectors that collect parameters, arguments, or statements
- Implement string interning with `once_cell::sync::Lazy<HashSet<Arc<str>>>` for identifiers and keywords
- Defer error formatting instead of using eager `format!` calls in hot loops

### 3. Limit Debug Output

**Prevent memory explosion in debug mode:**
- Truncate collections after 16 elements with a count of remaining items
- Limit long strings to first 128 bytes with ellipsis
- Ensure call frames are properly popped on return
- Clear retained data after generating debug reports
- Add doc comments explaining truncation limits

## Memory Profiling Results

### Heap Profiling Findings

Running the WFL interpreter on comprehensive test scripts revealed:

- **Peak memory usage**: ~10.7 GB
- **Total allocations**: Over 152 million
- **Primary culprits**: Parser expression routines and reference cycles

### Key Problem Areas

1. **Parser Expression Handling**
   - `parse_primary_expression` and `parse_binary_expression` dominated allocation profiles
   - Excessive AST node construction and token cloning
   - Unbounded vector reallocations during complex expression parsing

2. **Environment-Function Cycles**
   - Strong Rc pointers between Environment and FunctionValue created uncollectable cycles
   - Functions capturing outer scope environments couldn't be garbage collected
   - Memory remained allocated for the program duration

3. **Unbounded Data Structure Growth**
   - Debug call stack vectors growing without limits
   - Large runtime data structures without truncation
   - Verbose debug logging consuming excessive memory

## Best Practices

### Memory-Efficient Coding Patterns

1. **Use weak references for back-pointers**
2. **Reserve vector capacity when size is predictable**
3. **Implement string interning for repeated identifiers**
4. **Truncate debug output for large data structures**
5. **Clear temporary data structures promptly**
6. **Use references instead of cloning when possible**

### Performance Monitoring

1. **Regular heap profiling** with tools like `heaptrack`
2. **Automated memory regression tests**
3. **Monitor allocation patterns in CI/CD**
4. **Profile with realistic workloads**

### Code Review Guidelines

1. **Check for potential reference cycles**
2. **Verify proper cleanup of temporary allocations**
3. **Ensure debug output has reasonable limits**
4. **Review vector growth patterns**
5. **Validate weak reference usage**

## Tools and Techniques

### Profiling Tools
- **heaptrack**: Linux heap profiling
- **dhat**: Rust heap profiling (feature flag: `--features dhat-heap`)
- **valgrind**: Memory error detection
- **cargo bench**: Performance benchmarking

### Testing Strategies
- **Memory regression tests**: Automated tests that verify memory usage stays within bounds
- **Stress testing**: Large input files and complex nested structures
- **Long-running tests**: Verify no gradual memory leaks
- **Cycle detection**: Tests specifically for reference cycles

## Implementation Notes

### Weak Reference Pattern

```rust
// Before (creates cycle)
struct FunctionValue {
    env: Rc<RefCell<Environment>>,
}

// After (breaks cycle)
struct FunctionValue {
    env: Weak<RefCell<Environment>>,
}

// Usage
let weak_env = Rc::downgrade(&env);
let function = FunctionValue { env: weak_env };
```

### String Interning Pattern

```rust
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::sync::Arc;

static INTERNED_STRINGS: Lazy<Mutex<HashSet<Arc<str>>>> = 
    Lazy::new(|| Mutex::new(HashSet::new()));

fn intern_string(s: &str) -> Arc<str> {
    let mut set = INTERNED_STRINGS.lock().unwrap();
    if let Some(interned) = set.get(s) {
        Arc::clone(interned)
    } else {
        let arc_str: Arc<str> = Arc::from(s);
        set.insert(Arc::clone(&arc_str));
        arc_str
    }
}
```

### Debug Output Truncation

```rust
fn format_debug_list<T: Debug>(items: &[T]) -> String {
    const MAX_ITEMS: usize = 16;
    const MAX_STRING_LEN: usize = 128;
    
    if items.len() <= MAX_ITEMS {
        format!("{:?}", items)
    } else {
        let truncated = &items[..MAX_ITEMS];
        let remaining = items.len() - MAX_ITEMS;
        format!("{:?}... ({} more items)", truncated, remaining)
    }
}
```

## Historical Context

These guidelines were developed following a comprehensive memory leak investigation that identified critical performance issues in the WFL interpreter. The systematic approach to memory optimization has resulted in significant improvements in both memory usage and execution performance.

Regular application of these practices ensures WFL maintains its performance characteristics as the codebase evolves.
