# Getting Started with WFL MVC Framework

## Prerequisites

- WFL installed and working (version 26.1.19+)
- Basic understanding of WFL syntax
- Familiarity with MVC pattern

## Your First Application

### Step 1: Create a Simple API

Create `my_app.wfl`:

```wfl
display "Starting My First WFL MVC App"

listen on port 3000 as web_server
display "Server started on http://localhost:3000"

main loop:
    wait for request comes in on web_server as req

    check if path is equal to "/":
        respond to req with "Hello from WFL MVC!" and content_type "text/plain"
    otherwise check if path is equal to "/api/status":
        store status_json as "{\"status\":\"ok\",\"framework\":\"WFL MVC\"}"
        respond to req with status_json and content_type "application/json"
    otherwise:
        respond to req with "{\"error\":\"Not found\"}" and status 404 and content_type "application/json"
    end check
end loop
```

Run: `wfl my_app.wfl`

Test: `curl http://localhost:3000/api/status`

### Step 2: Add a Model

Create `models.wfl`:

```wfl
create container Task:
    property task_id: Number
    property task_name: Text
    property completed: Boolean

    action to_json:
        store json as "{\"id\":" with task_id
        change json to json with ",\"name\":" with stringify_json of task_name
        change json to json with ",\"completed\":" with completed
        change json to json with "}"
        return json
    end
end
```

Use in `my_app.wfl`:

```wfl
load module from "models.wfl"

// Create tasks list
store tasks_list as create list

create new Task as task1:
    task_id is 1
    task_name is "Learn WFL MVC"
    completed is yes
end

push with tasks_list and task1

// In request loop
check if path is equal to "/api/tasks":
    store tasks_json as "["
    store first as yes

    for each task in tasks_list:
        check if first is no:
            change tasks_json to tasks_json with ","
        end check
        change tasks_json to tasks_json with task.to_json()
        change first to no
    end for

    change tasks_json to tasks_json with "]"

    respond to req with tasks_json and content_type "application/json"
end check
```

### Step 3: Add Validation

Update `models.wfl`:

```wfl
create container Task:
    property task_id: Number
    property task_name: Text
    property completed: Boolean
    property is_valid: Boolean
    property errors_list: List

    action init_task:
        store is_valid as yes
        store errors_list as create list
    end

    action validate:
        store is_valid as yes
        store errors_list as create list

        store name_len as length of task_name
        check if name_len is less than 3:
            push with errors_list and "Task name must be at least 3 characters"
            store is_valid as no
        end check

        return is_valid
    end

    action to_json:
        // ... same as before
    end
end
```

Use validation:

```wfl
create new Task as new_task:
    task_id is 2
    task_name is "Do"  // Too short
    completed is no
    is_valid is yes
    errors_list is create list
end

new_task.init_task()

check if new_task.validate():
    push with tasks_list and new_task
    respond to req with new_task.to_json() and status 201 and content_type "application/json"
otherwise:
    store errors_json as stringify_json of new_task.get_errors()
    respond to req with errors_json and status 400 and content_type "application/json"
end check
```

### Step 4: Add Middleware

Create `middleware.wfl`:

```wfl
create container LoggerMiddleware:
    property request_count: Number

    action init_middleware:
        store request_count as 0
    end

    action handle needs req: Container, res: Container:
        store request_count as request_count plus 1
        display "[" with request_count with "] " with req.method_val with " " with req.path_val
        return yes  // Continue chain
    end
end
```

Use in app:

```wfl
load module from "middleware.wfl"

create new LoggerMiddleware as logger:
    request_count is 0
end

logger.init_middleware()

// In request loop (before routing)
logger.handle(req, res)
```

### Step 5: Add Sessions

```wfl
// Use framework session helper
load module from "wfl framework/helpers/sessions.wfl"

create new SessionStore as sessions:
    sessions_list is create list
    session_total is 0
end

sessions.init_manager()

// Create session
store new_session as sessions.create_session()
display "Session ID: " with new_session.session_id
display "CSRF Token: " with new_session.csrf_token

// Retrieve session
store user_session as sessions.get_session(session_id)
```

## Common Patterns

### JSON API Endpoint

```wfl
check if path is equal to "/api/users" and method is equal to "GET":
    store users_json as stringify_json of users_list
    respond to req with users_json and content_type "application/json"
end check
```

### Query Parameters

```wfl
// Extract query from path
// Example: /api/search?q=test&limit=10

// Parse query string manually
store query_part as ""
// Split path at "?" to get query string
// Then use: parse_query_string of query_part

store query_params as parse_query_string of query_part
// Access: query_params["q"], query_params["limit"]
```

### Error Responses

```wfl
check if not_found:
    store error_json as "{\"error\":\"Resource not found\",\"code\":404}"
    respond to req with error_json and status 404 and content_type "application/json"
end check
```

### Request Logging

```wfl
store timestamp as current time formatted as "yyyy-MM-dd HH:mm:ss"
display timestamp with " | " with method with " " with path with " | " with client_ip
```

## Testing Your Application

### Unit Testing Models

```wfl
// Create test model
create new UserModel as test_user:
    user_name is "Test"
    email is "test@example.com"
    age is 25
end

// Test validation
check if test_user.validate():
    display "✓ Validation passed"
otherwise:
    display "✗ Validation failed"
    display "Errors: " with test_user.get_errors()
end check

// Test JSON serialization
store json as test_user.to_json()
display "JSON: " with json
```

### Testing Routes

Use `curl` to test endpoints:

```bash
# GET request
curl http://localhost:3000/api/users

# POST request with JSON
curl -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/json" \
  -d '{"name":"Alice","email":"alice@example.com"}'

# Check headers
curl -v http://localhost:3000/api/status
```

## Troubleshooting

### Common Issues

**Issue**: "Variable 'X' is not defined"
- **Solution**: Check if using reserved keyword. See RESERVED_KEYWORDS.md

**Issue**: "Property 'X' has already been defined"
- **Solution**: Use `change X to value` instead of `store X as value` in loops

**Issue**: "'length' is not a function"
- **Solution**: Store result first: `store len as length of text` (don't use inline in expressions)

**Issue**: Container properties not persisting
- **Solution**: Ensure WFL version 26.1.19+ (property mutation fix included)

### Debugging Tips

1. **Add display statements** throughout your code
2. **Check container initialization**: Ensure all properties are initialized
3. **Validate JSON**: Use `parse_json` to verify your JSON is valid
4. **Test models separately**: Create models standalone before integrating
5. **Check HTTP status codes**: Verify responses return correct status

## Next Steps

1. **Explore Example Apps**:
   - Run `wfl "wfl framework/examples/blog_app/app.wfl"`
   - Run `wfl "wfl framework/examples/rest_api/app.wfl"`

2. **Read Documentation**:
   - `README.md` - Framework overview
   - `ARCHITECTURE.md` - Deep dive into architecture
   - `RESERVED_KEYWORDS.md` - Keywords to avoid

3. **Run Tests**:
   - Test files in `wfl framework/tests/`
   - Learn from working examples

4. **Build Your App**:
   - Start with models (data + validation)
   - Add controllers (actions)
   - Add views (rendering)
   - Wire up routing
   - Add middleware/plugins as needed

## Resources

- **Example Apps**: See `examples/blog_app/` and `examples/rest_api/`
- **Tests**: See `tests/` directory for usage examples
- **Core Components**: See `core/`, `mvc/`, `middleware/`, `plugins/`

## Support

For WFL language issues, see the main WFL documentation in `Docs/`.

For framework issues discovered during development, see `FRAMEWORK_PROPERTY_MUTATION_ISSUE.md` for the major bug that was fixed.

---

**Happy coding with WFL MVC!** 🚀
