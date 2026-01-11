# WFL MVC Web Framework

A full-featured MVC web framework written entirely in **WFL (WebFirst Language)** with natural language syntax. Build web applications and REST APIs with readable, intuitive code.

## Features

- 🏗️ **MVC Architecture** - Models, Views, and Controllers with clean separation
- 🛤️ **Routing System** - Route registration with pattern support
- 🔌 **Plugin System** - Extensible with lifecycle hooks (before_request, after_request, etc.)
- 🔧 **Middleware Pipeline** - CORS, Logging, Error Handling, Authentication
- 🍪 **Request/Response Helpers** - Query params, cookies, form data, sessions
- 🔐 **Security** - UUID generation, CSRF tokens, session management
- 📦 **JSON Support** - Full JSON parsing and serialization
- ✨ **Natural Language Syntax** - Code that reads like English

## Quick Start

### Hello World API

```wfl
display "Starting WFL MVC App..."

listen on port 3000 as web_server

main loop:
    wait for request comes in on web_server as req

    check if path is equal to "/api/hello":
        store response_json as "{\"message\":\"Hello from WFL MVC!\"}"
        respond to req with response_json and content_type "application/json"
    otherwise:
        respond to req with "{\"error\":\"Not found\"}" and status 404 and content_type "application/json"
    end check

    // Stop after 10 requests (remove for production)
    break
end loop
```

Run: `wfl app.wfl`

### Using the Framework

```wfl
// Load framework components
load module from "wfl framework/core/router.wfl"
load module from "wfl framework/mvc/model.wfl"
load module from "wfl framework/mvc/controller.wfl"

// Create a model
create new UserModel as user:
    user_name is "Alice"
    email is "alice@example.com"
    age is 30
end

check if user.validate():
    store json as user.to_json()
    display "Valid user: " with json
otherwise:
    store errors as user.get_errors()
    display "Validation errors: " with length of errors
end check
```

## Project Structure

```
wfl framework/
├── core/                    # Core framework components
│   ├── application.wfl      # Main app bootstrap
│   ├── router.wfl          # Route registry and matching
│   ├── request.wfl         # Request wrapper
│   ├── response.wfl        # Response builder
│   ├── middleware.wfl      # Middleware base
│   ├── plugin_interface.wfl # Plugin base
│   └── plugin_manager.wfl  # Plugin coordinator
├── routing/                 # Route matching utilities
│   ├── route_compiler.wfl  # Pattern compilation
│   └── route_matcher.wfl   # Route matching
├── middleware/              # Built-in middleware
│   ├── cors.wfl            # CORS headers
│   ├── logging.wfl         # Request logging
│   └── error_handler.wfl   # Error handling
├── helpers/                 # Helper functions
│   └── sessions.wfl        # Session management
├── plugins/                 # Built-in plugins
│   ├── cors_plugin.wfl     # CORS plugin
│   ├── auth_plugin.wfl     # Authentication
│   └── logger_plugin.wfl   # Request logger
├── mvc/                     # MVC components
│   ├── model.wfl           # Base model + UserModel
│   ├── view.wfl            # Views (HTML, JSON)
│   └── controller.wfl      # Base controller + examples
├── config/                  # Configuration
│   └── plugins.wfl         # Plugin settings
├── examples/                # Example applications
│   ├── blog_app/           # Blog application
│   └── rest_api/           # REST API example
└── tests/                   # Framework tests
    └── test_*.wfl          # Comprehensive tests
```

## Example Applications

### Blog Application

Complete blog with posts, validation, and CRUD operations.

**Run**: `wfl "wfl framework/examples/blog_app/app.wfl"`

**Endpoints**:
- `GET /` - Welcome page
- `GET /api/posts` - List all posts
- `GET /api/posts/1` - Get post by ID
- `POST /api/posts` - Create new post

**Features**:
- PostModel with validation (title, content, author)
- BlogController with stateful posts list
- JSON API responses
- Error handling

### REST API

RESTful API with users and health checks.

**Run**: `wfl "wfl framework/examples/rest_api/app.wfl"`

**Endpoints**:
- `GET /` - API info
- `GET /api/status` - Health check
- `GET /api/users` - List all users
- `GET /api/users/:id` - Get user by ID
- `POST /api/users` - Create user

**Features**:
- ApiUserModel with validation
- ApiResponse wrapper
- HTTP status codes (200, 201, 404)
- JSON responses

## Core Components

### Models

Data models with validation and JSON serialization:

```wfl
create container UserModel:
    property user_name: Text
    property email: Text
    property age: Number
    property is_valid: Boolean
    property errors_list: List

    action validate:
        store name_len as length of user_name
        check if name_len is less than 3:
            push with errors_list and "Name too short"
            store is_valid as no
        end check

        check if email contains "@":
        otherwise:
            push with errors_list and "Invalid email"
            store is_valid as no
        end check

        return is_valid
    end

    action to_json:
        store json as "{\"name\":" with stringify_json of user_name
        change json to json with ",\"email\":" with stringify_json of email
        change json to json with ",\"age\":" with age with "}"
        return json
    end
end
```

### Views

HTML and JSON view rendering:

```wfl
// HTML View
create new HtmlView as page:
    page_title is "Welcome"
    page_content is "<h1>Hello!</h1>"
end

page.render()  // Full HTML document

// JSON View
create new JsonView as api:
    json_data is users_list
end

api.render()  // JSON string
```

### Controllers

Action handlers for routes:

```wfl
create container UserController:
    property controller_name: Text

    action index_action needs req: Container, res: Container:
        // List users
        store users_json as "[...]"
        res.set_cont(users_json)
        res.set_stat(200)
    end

    action show_action needs req: Container, res: Container, user_id: Text:
        // Show single user
        store user_json as "{...}"
        res.set_cont(user_json)
        res.set_stat(200)
    end
end
```

### Router

Route registration and matching:

```wfl
create new Router as router:
    routes is create list
    routes_count is 0
end

router.add_route("GET", "/api/users", "UserController.index")
router.add_route("GET", "/api/users/:id", "UserController.show")
router.add_route("POST", "/api/users", "UserController.create")
```

### Middleware

Request/response processing pipeline:

```wfl
// CORS Middleware
create new CorsMiddleware as cors:
    allowed_origins is "*"
    allowed_methods is "GET,POST,PUT,DELETE"
end

cors.handle(request, response)

// Logger Middleware
create new LoggingMiddleware as logger:
    request_count is 0
end

logger.handle(request, response)
// Logs: timestamp | method | path | ip
```

### Plugins

Extensible plugin system:

```wfl
create new LoggerPlugin as logger:
    plugin_name is "Logger"
    plugin_enabled is yes
    request_total is 0
end

logger.initialize()
logger.on_request_complete(request, response)
// request_total increments with each request
```

## Standard Library Functions

### JSON
- `parse_json(text)` - Parse JSON to WFL objects/lists
- `stringify_json(value)` - Convert to JSON string
- `stringify_json_pretty(value)` - Pretty-print JSON

### Request Parsing
- `parse_query_string(query)` - Parse ?page=1&limit=10
- `parse_cookies(header)` - Parse cookie header
- `parse_form_urlencoded(body)` - Parse form data

### Security
- `generate_uuid()` - Generate UUID v4 for sessions
- `generate_csrf_token()` - Generate 256-bit secure token

### Headers
- `header "Authorization" of request` - Access HTTP headers

## Requirements

- **WFL**: Version 26.1.19 or later
- **WFL Features Used**:
  - Containers (OOP)
  - Actions with typed parameters
  - Lists and Objects
  - Module system
  - Built-in web server
  - Pattern matching
  - Async support

## Installation

1. Ensure WFL is installed and working
2. Clone or download the `wfl framework/` directory
3. Run example applications to test

## Testing

Run all framework tests:

```bash
# Core tests
wfl "wfl framework/tests/test_routing_simple.wfl"
wfl "wfl framework/tests/test_middleware_simple.wfl"
wfl "wfl framework/tests/test_plugins_simple.wfl"
wfl "wfl framework/tests/test_mvc_simple.wfl"
wfl "wfl framework/tests/test_sessions_simple.wfl"
wfl "wfl framework/tests/test_example_apps_simple.wfl"

# Helper tests
wfl "TestPrograms/test_json_and_headers.wfl"
wfl "TestPrograms/test_request_helpers.wfl"
wfl "TestPrograms/test_container_property_mutation.wfl"
```

All tests should pass ✅

## Known Limitations

### Reserved Keywords

WFL has many reserved keywords. Avoid using these as property names:
- `port`, `data`, `content`, `status`, `count`, `total`
- `start`, `handler`, `pattern`, `register`, `now`
- `response`, `request` (context-dependent)

**Use instead**: `port_number`, `session_data`, `response_text`, `status_code`, `value`, `session_total`, `run_server`, `handler_name`, `route_pattern`, `add_route`, `current_time`

### Best Practices

1. **Property Names**: Use underscores for reserved words (`user_name` not `name`)
2. **Variable Scoping**: Use `change` for reassignment in loops, `store` for first assignment
3. **Length Function**: Store result first: `store len as length of text` (don't use inline)
4. **Container Actions**: Properties modified in actions now persist (property mutation fixed!)

## Architecture

```
Request → Application
           ↓
       Plugin Manager (before_request hooks)
           ↓
       Middleware Chain
           ↓
       Router (match route)
           ↓
       Controller Action
           ↓
       Model (data + validation)
           ↓
       View (render HTML/JSON)
           ↓
       Plugin Manager (after_request hooks)
           ↓
       Response → Client
           ↓
       Plugin Manager (request_complete hooks)
```

## Contributing

The framework was built to identify and fix WFL language issues. During development, we:

1. ✅ **Fixed HTTP header access** - Headers now return actual values
2. ✅ **Added JSON stdlib** - parse_json, stringify_json functions
3. ✅ **Fixed property mutation** - Container properties now persist in actions
4. ✅ **Added request helpers** - Query, cookie, form parsing
5. ✅ **Added security functions** - UUID and CSRF token generation

See `FRAMEWORK_PROPERTY_MUTATION_ISSUE.md` for details on the major bug fix.

## License

Part of the WFL project. See main WFL repository for license.

## Credits

Built with ❤️ using WFL's natural language syntax.

**Framework Stats**:
- 58 files
- ~4,670 lines of WFL code
- 9 sprints completed
- 2 complete example applications
- Full test coverage

---

**Ready to build web applications with WFL!** 🚀
