# Cache Module

The Cache module provides a simple **in-memory TTL (time-to-live) cache**. It's
handy for holding onto results that are expensive to compute but only need to be
fresh for a while — a rendered page, an API response, a database lookup — so you
can serve them again quickly without redoing the work.

A cache is created with `create_cache`, which gives you back a **cache handle**.
You pass that handle to the other functions to store and look up values. Each
value is stored with a lifetime in seconds; once that time passes, the value is
treated as if it were never there.

Expiry is **lazy**: an expired entry is dropped the next time you look it up (or
when you call `cache_size`), so there is no background thread and no surprise
pauses. Timing uses a monotonic clock, so entries expire on schedule even if the
system wall-clock is adjusted.

## Functions

### create_cache

**Purpose:** Create a new, empty cache and return its handle.

**Signature:**
```wfl
create_cache
```

**Parameters:** None.

**Returns:** Number - A handle identifying the new cache.

**Example:**
```wfl
store pages as create_cache
```

---

### cache_set

**Purpose:** Store a value under a key, with a time-to-live.

**Signature:**
```wfl
cache_set of cache and key and value and ttl_seconds
```

**Parameters:**
- `cache` (Number) - A cache handle from `create_cache`.
- `key` (Text) - The key to store the value under.
- `value` (any) - The value to cache. Any WFL value works.
- `ttl_seconds` (Number) - How long the entry stays valid, in seconds. A value of
  `0` (or negative) means **never expires**.

**Returns:** Nothing.

**Example:**
```wfl
cache_set of pages and "/home" and rendered_html and 60   // valid for 60 seconds
cache_set of pages and "config" and settings and 0        // never expires
```

**Notes:** Setting a key that already exists replaces both its value and its TTL.

---

### cache_get

**Purpose:** Look up a value by key.

**Signature:**
```wfl
cache_get of cache and key
```

**Parameters:**
- `cache` (Number) - A cache handle.
- `key` (Text) - The key to look up.

**Returns:** The stored value, or `nothing` if the key is absent or has expired.

**Example:**
```wfl
store hit as cache_get of pages and "/home"
check if hit is nothing:
    store hit as render_home_page
    cache_set of pages and "/home" and hit and 60
end check
display hit
```

---

### cache_has

**Purpose:** Check whether a key is present and unexpired.

**Signature:**
```wfl
cache_has of cache and key
```

**Parameters:**
- `cache` (Number) - A cache handle.
- `key` (Text) - The key to check.

**Returns:** Boolean - `yes` if the key has a live value, otherwise `no`.

**Example:**
```wfl
check if cache_has of pages and "/home":
    display "serving cached copy"
end check
```

---

### cache_delete

**Purpose:** Remove a key from the cache.

**Signature:**
```wfl
cache_delete of cache and key
```

**Parameters:**
- `cache` (Number) - A cache handle.
- `key` (Text) - The key to remove.

**Returns:** Boolean - `yes` if a live entry was removed, `no` if the key was
absent or already expired.

**Example:**
```wfl
cache_delete of pages and "/home"
```

---

### cache_clear

**Purpose:** Remove every entry from the cache.

**Signature:**
```wfl
cache_clear of cache
```

**Parameters:**
- `cache` (Number) - A cache handle.

**Returns:** Nothing.

**Example:**
```wfl
cache_clear of pages
```

---

### cache_size

**Purpose:** Count the live (unexpired) entries in the cache.

**Signature:**
```wfl
cache_size of cache
```

**Parameters:**
- `cache` (Number) - A cache handle.

**Returns:** Number - How many entries are currently valid.

**Example:**
```wfl
display "cached pages: " with cache_size of pages
```

**Notes:** Calling `cache_size` also sweeps out any expired entries, so calling it
periodically keeps memory bounded even for keys you never look up again.

## A complete pattern: cache-or-compute

```wfl
store pages as create_cache

define action called get_page with path:
    store cached as cache_get of pages and path
    check if cached is nothing:
        // Cache miss: do the expensive work once, then remember it.
        store fresh as render_page with path
        cache_set of pages and path and fresh and 60
        give back fresh
    otherwise:
        give back cached
    end check
end action
```

## See Also

- [Web Servers](../04-advanced-features/web-servers.md) — a natural place to cache
  rendered responses.
