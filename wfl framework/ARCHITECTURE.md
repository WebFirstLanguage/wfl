# WFL MVC Framework - Architecture Guide

## Overview

The WFL MVC Framework is built using WFL's container system (OOP), providing a clean MVC architecture with plugin support and middleware pipeline.

## Request Lifecycle

```
1. Client Request
   ↓
2. Web Server (listen on port)
   ↓
3. Application receives request
   ↓
4. Plugin Manager: execute_before_request()
   - Plugins can short-circuit (auth, rate limiting)
   ↓
5. Middleware Chain
   - CORS, Logging, Error Handler
   ↓
6. Router: match_route(method, path)
   - Find matching route
   - Extract parameters (:id)
   ↓
7. Controller Action
   - Process request
   - Load/Create models
   - Validate data
   ↓
8. Model Layer
   - Data validation
   - Business logic
   ↓
9. View Layer
   - Render HTML/JSON
   ↓
10. Plugin Manager: execute_after_request()
    - Modify response (add headers)
    ↓
11. Send Response to Client
    ↓
12. Plugin Manager: execute_request_complete()
    - Logging, cleanup, metrics
```

## Component Architecture

### Core Layer

**Application Container**:
- Central coordinator
- Manages web server lifecycle
- Orchestrates router, middleware, plugins
- Main request loop

**Router Container**:
- Route registry (method + pattern + handler)
- Route matching
- Parameter extraction (:id, :slug)

**Request/Response Containers**:
- Wrap HTTP request/response
- Typed properties
- Helper methods

### MVC Layer

**Model**:
- Data representation
- Validation logic
- Error tracking (errors_list with property mutation)
- JSON serialization (to_json)

**View**:
- Template rendering
- HTML generation
- JSON formatting

**Controller**:
- Action methods (index, show, create, etc.)
- Request handling
- Model coordination
- Response building

### Middleware Layer

**Middleware Pipeline**:
- Sequential execution
- Short-circuit support (return no to stop)
- Request/response modification

**Built-in Middleware**:
- CORS: Add Access-Control-* headers
- Logging: Request logging with timestamps
- ErrorHandler: Global error catching

### Plugin Layer

**Plugin System**:
- Container-based plugins
- Lifecycle hooks (6 hooks)
- Stateful plugins (property mutation enabled)
- Enable/disable individual plugins

**Lifecycle Hooks**:
1. `initialize()` - Plugin setup
2. `before_request(req)` - Pre-routing
3. `after_request(req, res)` - Post-routing, pre-response
4. `on_request_complete(req, res)` - Post-response
5. `on_error(error_msg)` - Error handling
6. `shutdown()` - Cleanup

## Data Flow

### Request Processing

```wfl
main loop:
    wait for request comes in on web_server as incoming_request

    // 1. Wrap request
    create new Request as req:
        method_val is method
        path_val is path
    end

    // 2. Create response builder
    create new Response as res:
        status_code is 200
    end

    // 3. Plugin hooks (before)
    plugin_manager.execute_before_request(req)

    // 4. Middleware chain
    execute_middleware_chain(middleware_list, req, res)

    // 5. Route matching
    store route as router.match_route(method, path)

    // 6. Controller action
    controller.action(req, res)

    // 7. Plugin hooks (after)
    plugin_manager.execute_after_request(req, res)

    // 8. Send response
    respond to incoming_request with res.content_val
        and status res.status_code
        and content_type res.content_type_val

    // 9. Plugin hooks (complete)
    plugin_manager.execute_request_complete(req, res)
end loop
```

### Model Validation Flow

```wfl
// 1. Create model
create new UserModel as user:
    user_name is input_name
    email is input_email
end

// 2. Validate
check if user.validate():
    // 3a. Valid - proceed
    store json as user.to_json()
    res.set_cont(json)
    res.set_stat(200)
otherwise:
    // 3b. Invalid - return errors
    store errors as user.get_errors()
    res.set_cont(stringify_json of errors)
    res.set_stat(400)
end check
```

## Property Mutation Fix

**Critical Feature**: Container properties now persist when modified in actions.

### Before Fix (BROKEN):
```wfl
action increment:
    store count as count plus 1  // Modified locally only
end

my_counter.increment()
my_counter.count  // Still 0 ❌
```

### After Fix (WORKING):
```wfl
action increment:
    store count as count plus 1  // Persists to container!
end

my_counter.increment()
my_counter.count  // Now 1 ✅
```

**Impact**:
- ✅ Middleware request counters work
- ✅ Session state persists
- ✅ Model error lists accumulate
- ✅ Plugin state tracking works
- ✅ Any stateful container operations work

**Implementation**: Modified `src/interpreter/mod.rs` to write back property values from method environment to container instance after action execution.

## Container System

WFL containers provide OOP capabilities:

```wfl
create container MyContainer:
    property prop_name: Type

    action my_action needs param: Type:
        store prop_name as new_value  // NOW PERSISTS!
    end
end

create new MyContainer as instance:
    prop_name is initial_value
end

instance.my_action(argument)
```

**Features**:
- Typed properties
- Action methods with parameters
- Inheritance (extends keyword)
- Interfaces (implements keyword)
- Property mutation support

## Session Management

```wfl
// Session with UUID and CSRF
create new Session as session:
    session_id is ""
    csrf_token is ""
    created_at is 0
end

session.initialize()
// session_id: "f5e7b305-fe9e-44af-9992-b7ba2ceaac00"
// csrf_token: "a342f0a1ede6a8a557efdb478110fe4c..."

// Session store
create new SessionStore as sessions:
    sessions_list is create list
    session_total is 0
end

store my_session as sessions.create_session()
store retrieved as sessions.get_session(session_id)
```

## Security Considerations

1. **CSRF Protection**: Use `generate_csrf_token()` for forms
2. **Session IDs**: Use `generate_uuid()` for unique sessions
3. **Input Validation**: Always validate user input in models
4. **SQL Injection**: N/A (no SQL support yet)
5. **XSS**: Sanitize HTML output in views
6. **Headers**: Use CORS plugin for cross-origin requests

## Performance

The framework is built on WFL's async runtime (Tokio):
- Non-blocking I/O
- Async request handling
- Efficient container cloning (Rc/RefCell)
- Pattern matching with bytecode VM

## Extensibility

### Creating Custom Middleware

```wfl
create container CustomMiddleware:
    property middleware_name: Text
    property custom_state: Number

    action handle needs req: Container, res: Container:
        store custom_state as custom_state plus 1
        display "Middleware executed " with custom_state with " times"
        return yes  // Continue chain
    end
end
```

### Creating Custom Plugins

```wfl
create container CustomPlugin:
    property plugin_name: Text
    property plugin_enabled: Boolean

    action before_request needs req: Container:
        // Pre-processing
        return nothing  // Continue
    end

    action after_request needs req: Container, res: Container:
        // Post-processing
        return res
    end
end
```

### Creating Custom Models

```wfl
create container ProductModel:
    property product_id: Number
    property product_name: Text
    property price: Number
    property is_valid: Boolean

    action validate:
        store is_valid as yes

        store name_len as length of product_name
        check if name_len is less than 3:
            store is_valid as no
        end check

        check if price is less than 0:
            store is_valid as no
        end check

        return is_valid
    end

    action to_json:
        store json as "{\"id\":" with product_id
        change json to json with ",\"name\":" with stringify_json of product_name
        change json to json with ",\"price\":" with price
        change json to json with "}"
        return json
    end
end
```

## Design Patterns

### Repository Pattern (with Session Store)

```wfl
create container UserRepository:
    property users_list: List

    action find_by_id needs user_id: Number:
        for each user in users_list:
            check if user.user_id is equal to user_id:
                return user
            end check
        end for
        return nothing
    end

    action save_user needs user: Container:
        push with users_list and user
    end
end
```

### Factory Pattern

```wfl
create container ModelFactory:
    action create_user needs username: Text:
        create new UserModel as user:
            user_name is username
        end
        user.init_model()
        return user
    end
end
```

### Observer Pattern (with Events)

WFL containers support events natively - can be used for plugin communication.

## Future Enhancements

Potential additions:
- Database ORM layer
- Advanced template engine (loops, conditionals in templates)
- WebSocket support
- File upload handling (multipart/form-data)
- Authentication providers (OAuth, JWT parsing)
- Rate limiting plugin
- Caching layer
- Asset pipeline
- CLI scaffolding tools

## Debugging

Enable debug mode in WFL:
```bash
wfl --debug myapp.wfl
```

Check logs:
```wfl
// Add logging throughout your code
display "Request: " with method with " " with path
display "Model valid: " with is_valid
display "Response: " with res.status_code
```

## Conclusion

The WFL MVC Framework demonstrates that WFL is production-ready for web development. The natural language syntax makes code highly readable while maintaining full functionality.

**Key Achievement**: Fixed critical property mutation bug in WFL interpreter, enabling stateful OOP operations essential for web frameworks.
