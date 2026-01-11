# WFL Reserved Keywords - Framework Reference

This document lists reserved keywords discovered during WFL MVC framework development and provides safe alternatives.

## Reserved Keywords to Avoid

These keywords cannot be used as property names, variable names, or action names in certain contexts:

### Common Property Name Conflicts

| Keyword | Context | Use Instead | Example |
|---------|---------|-------------|---------|
| `port` | Property name | `port_number`, `port_num` | `property port_number: Number` |
| `data` | Property name | `session_data`, `user_data`, `model_data` | `property session_data: List` |
| `content` | Variable name | `response_text`, `html_content`, `json_content` | `store response_text as "..."` |
| `status` | Variable/Property | `status_code`, `status_text` | `property status_code: Number` |
| `count` | Property/Variable | `value`, `counter`, `total_count`, `item_count` | `property value: Number` |
| `total` | Variable | `session_total`, `request_total`, `item_total` | `property session_total: Number` |
| `now` | Variable | `current_time`, `timestamp` | `store current_time as current time in milliseconds` |

### Action Name Conflicts

| Keyword | Context | Use Instead | Example |
|---------|---------|-------------|---------|
| `start` | Action name | `run_server`, `start_app`, `begin` | `action run_server:` |
| `register` | Action name | `add_route`, `add_plugin`, `add_middleware` | `action add_route needs ...` |
| `handler` | Parameter name | `handler_name`, `action_name` | `action add_route needs handler_name: Text:` |
| `pattern` | Parameter name | `route_pattern`, `url_pattern` | `action match needs route_pattern: Text:` |

### Variable Name Conflicts

| Keyword | Context | Use Instead | Example |
|---------|---------|-------------|---------|
| `response` | Variable | `api_response`, `http_response`, `res` | `store api_response as ...` |
| `request` | Variable (sometimes) | `http_request`, `incoming_req`, `req` | `store req as ...` |
| `store` | Variable | `session_store`, `data_store` | `create new SessionStore as session_store:` |
| `method` | Property (sometimes) | `http_method`, `request_method`, `method_val` | `property method_val: Text` |
| `path` | Property (sometimes) | `request_path`, `url_path`, `path_val` | `property path_val: Text` |

### Special Cases

| Keyword | Issue | Solution |
|---------|-------|----------|
| `length` | Cannot use inline in conditionals | Store first: `store len as length of text` |
| `and` | Use commas in action params | `action foo needs a: Text, b: Text:` not `a: Text and b: Text` |
| `with` | Cannot use in action params | Use `needs` keyword instead |

## Safe Naming Conventions

### Properties

✅ **Good**:
```wfl
property port_number: Number
property session_data: List
property status_code: Number
property request_count: Number
property user_name: Text
property session_total: Number
```

❌ **Bad**:
```wfl
property port: Number        // Reserved
property data: List          // Reserved
property status: Number      // Reserved
property count: Number       // Reserved
property name: Text          // May conflict
property total: Number       // May conflict
```

### Action Parameters

✅ **Good**:
```wfl
action add_route needs method: Text, route_pattern: Text, handler_name: Text:
action process needs req: Container, res: Container:
action configure needs origins: Text, methods: Text:
```

❌ **Bad**:
```wfl
action add_route needs method and pattern and handler:  // Wrong syntax
action register needs ...:                               // Reserved keyword
action start needs ...:                                  // Reserved keyword
```

### Variable Names

✅ **Good**:
```wfl
store api_response as create_response()
store current_time as current time in milliseconds
store session_store as create new SessionStore
store method_val as method
```

❌ **Bad**:
```wfl
store response as ...   // May conflict
store now as ...        // Reserved (builtin function)
store store as ...      // Reserved keyword
store method as ...     // Contextual reserved
```

## Complete List of Keywords to Avoid

Based on framework development, avoid these in your code:

### WFL Language Keywords
- `action`, `check`, `otherwise`, `end`, `loop`, `for`, `each`, `in`
- `create`, `container`, `property`, `store`, `change`, `display`
- `if`, `is`, `as`, `to`, `from`, `with`, `and`, `or`, `not`
- `wait`, `listen`, `respond`, `break`, `continue`, `return`
- `try`, `catch`, `call`, `load`, `module`

### WFL Builtin Functions (60+)
- `now`, `today`, `time`, `date`
- `length`, `contains`, `substring`
- `push`, `pop`, `count`, `total`, `size`
- `random`, `parse`, `stringify`, `typeof`
- Many more - see WFL documentation

### Context-Sensitive Keywords
These MAY conflict depending on usage:
- `method`, `path`, `body`, `headers` (in web server context)
- `request`, `response`, `server` (in HTTP context)
- `file`, `directory`, `error`, `result`
- `session`, `user`, `admin`, `role`

## Validation Strategy

Before using a variable/property name:

1. **Check WFL keywords**: Is it in the language syntax?
2. **Check builtins**: Is it a stdlib function?
3. **Use underscores**: When in doubt, add suffix: `user_name`, `user_data`
4. **Be specific**: `session_total` is better than `total`
5. **Test early**: Create simple test to verify name works

## Examples from Framework

### Router Container

```wfl
// ❌ Original (broke)
action register needs method and pattern and handler:

// ✅ Fixed
action add_route needs method: Text, route_pattern: Text, handler_name: Text:
```

### Application Container

```wfl
// ❌ Original (broke)
property port: Number
action start:

// ✅ Fixed
property port_number: Number
action run_server:
```

### Session Container

```wfl
// ❌ Original (broke)
property data: Container
store now as current time

// ✅ Fixed
property session_data: List
store current_time as current time in milliseconds
```

### Response Container

```wfl
// ❌ Original (broke)
property content: Text
property status: Number

// ✅ Fixed
property content_val: Text
property status_code: Number
```

## Quick Reference Card

**When you see these errors**, check for reserved keywords:

- "Unexpected token" → Likely reserved keyword
- "Expected identifier, found Keyword..." → Reserved keyword
- "Property definitions require a name" → Reserved keyword as property
- "Variable 'X' is not a function" → Builtin function name conflict

**Safe Prefixes/Suffixes**:
- `_val`, `_text`, `_num`, `_code`, `_total`, `_count`, `_data`, `_list`
- `user_`, `session_`, `request_`, `response_`, `api_`, `temp_`

## Testing for Reserved Keywords

```wfl
// Test if a name is safe
create container TestContainer:
    property my_property_name: Text  // Replace with name to test
end

// If this parses, the name is safe
```

## Updates

This list is based on WFL MVC framework development (2026-01-10). As WFL evolves, more keywords may be added. Always test your property/variable names before extensive use.

## Summary

**Golden Rules**:
1. **Use descriptive names**: `user_name` instead of `name`
2. **Add underscores**: `session_data` instead of `data`
3. **Be specific**: `status_code` instead of `status`
4. **Test first**: Create small test if unsure
5. **Use commas**: Action params use commas, not `and`
6. **Store then use**: Don't use `length of X` inline, store first

Following these guidelines will prevent most parsing and runtime errors!
