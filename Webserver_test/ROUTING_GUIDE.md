# WFL Web Server Routing Guide

This guide demonstrates how to implement routing in WFL web servers, allowing you to load different `.wfl` modules based on the request path and HTTP method.

## Core Concept

**Module-Based Routing**: Each route loads a separate `.wfl` file that handles the request. The route module has access to request information through variables set in the parent scope.

## Available Request Variables

When a route module is loaded, the following variables are available:

- `REQUEST_METHOD` - HTTP method (GET, POST, PUT, DELETE, etc.)
- `REQUEST_PATH` - Request path (e.g., "/api/status")
- `REQUEST_BODY` - Request body content
- `REQUEST_HEADERS` - Request headers object
- `REQUEST_CLIENT_IP` - Client IP address
- `REQUEST_OBJECT` - The WFL request object for responding

## Basic Routing Pattern

```wfl
wait for request comes in on web_server as incoming_request

// Extract request information
store request_method as method of incoming_request
store request_path as path of incoming_request
store request_body as body of incoming_request
store request_headers as headers of incoming_request
store request_client_ip as client_ip of incoming_request

// Route based on path
check if request_path is equal to "/":
    check if request_method is equal to "GET":
        // Set context variables
        store REQUEST_METHOD as request_method
        store REQUEST_PATH as request_path
        store REQUEST_BODY as request_body
        store REQUEST_HEADERS as request_headers
        store REQUEST_CLIENT_IP as request_client_ip
        store REQUEST_OBJECT as incoming_request

        // Load the route module
        load module from "routes/home.wfl"
    otherwise:
        respond to incoming_request with "Method not allowed" and status 405
    end check

otherwise check if request_path is equal to "/api/status":
    check if request_method is equal to "GET":
        // Set context and load module
        store REQUEST_METHOD as request_method
        store REQUEST_PATH as request_path
        store REQUEST_OBJECT as incoming_request
        load module from "routes/api_status.wfl"
    end check

otherwise:
    // 404 Not Found
    respond to incoming_request with "404 Not Found" and status 404
end check
```

## Route Module Example

**routes/home.wfl**:
```wfl
// This module has access to REQUEST_* variables
store html_content as "<!DOCTYPE html>
<html>
<head><title>Home</title></head>
<body>
    <h1>Welcome!</h1>
    <p>Method: " with REQUEST_METHOD with "</p>
    <p>Path: " with REQUEST_PATH with "</p>
    <p>Client IP: " with REQUEST_CLIENT_IP with "</p>
</body>
</html>"

respond to REQUEST_OBJECT with html_content and content_type "text/html"
```

**routes/api_status.wfl**:
```wfl
// Build JSON response
store json_response as "{
    \"status\": \"running\",
    \"method\": \"" with REQUEST_METHOD with "\",
    \"path\": \"" with REQUEST_PATH with "\"
}"

respond to REQUEST_OBJECT with json_response and content_type "application/json"
```

**routes/api_echo.wfl**:
```wfl
// Echo back the request body
check if REQUEST_BODY is equal to "":
    store error_json as "{\"error\": \"No body provided\"}"
    respond to REQUEST_OBJECT with error_json and status 400 and content_type "application/json"
otherwise:
    store echo_json as "{
        \"echo\": \"" with REQUEST_BODY with "\",
        \"length\": " with length of REQUEST_BODY with "
    }"
    respond to REQUEST_OBJECT with echo_json and content_type "application/json"
end check
```

## Complete Working Example

See `minimal_routing_demo.wfl` for a complete, working example with:
- Home page route (GET /)
- API status endpoint (GET /api/status)
- Echo endpoint (POST /api/echo)
- 404 handling for unknown routes
- Method validation (405 for wrong methods)

## Running the Demo

```bash
wfl Webserver_test/minimal_routing_demo.wfl
```

Then access:
- http://localhost:8080/ - Home page
- http://localhost:8080/api/status - JSON status
- POST to http://localhost:8080/api/echo with body - Echo response

## Advanced Patterns

### Helper Action for Context Setup

To reduce repetition, you can create an action that sets up the request context:

```wfl
define action called setup_request_context with parameters req_method and req_path and req_body and req_ip and req_obj:
    store REQUEST_METHOD as req_method
    store REQUEST_PATH as req_path
    store REQUEST_BODY as req_body
    store REQUEST_CLIENT_IP as req_ip
    store REQUEST_OBJECT as req_obj
end action

// Then use it in your routing:
check if request_path is equal to "/":
    call setup_request_context with request_method and request_path and request_body and request_client_ip and incoming_request
    load module from "routes/home.wfl"
end check
```

### Pattern Matching for Dynamic Routes

For routes with parameters (e.g., `/users/123`), you can use pattern matching:

```wfl
check if request_path starts with "/users/":
    // Extract user ID
    store user_id_start as 7  // length of "/users/"
    store user_id as substring of request_path from user_id_start to length of request_path

    // Set context with extracted parameter
    store REQUEST_USER_ID as user_id
    store REQUEST_METHOD as request_method
    store REQUEST_OBJECT as incoming_request

    load module from "routes/user_profile.wfl"
end check
```

**routes/user_profile.wfl**:
```wfl
store profile_html as "<html>
<body>
    <h1>User Profile</h1>
    <p>User ID: " with REQUEST_USER_ID with "</p>
</body>
</html>"

respond to REQUEST_OBJECT with profile_html and content_type "text/html"
```

### Static File Serving

```wfl
check if request_path starts with "/static/":
    // Extract filename
    store filename_start as 8  // length of "/static/"
    store filename as substring of request_path from filename_start to length of request_path
    store full_path as "public/" with filename

    check if file exists at full_path:
        read file at full_path into file_contents

        // Determine MIME type based on extension
        store mime_type as "text/html"
        check if filename ends with ".css":
            store mime_type as "text/css"
        otherwise check if filename ends with ".js":
            store mime_type as "application/javascript"
        otherwise check if filename ends with ".png":
            store mime_type as "image/png"
        end check

        respond to incoming_request with file_contents and content_type mime_type
    otherwise:
        respond to incoming_request with "File not found" and status 404
    end check
end check
```

## Best Practices

1. **Keep route modules focused** - Each module should handle one route/endpoint
2. **Use consistent naming** - `routes/api_status.wfl` for `/api/status`
3. **Always validate input** - Check REQUEST_BODY, parameters, etc.
4. **Set appropriate content types** - `text/html` for HTML, `application/json` for JSON
5. **Handle errors gracefully** - Return proper HTTP status codes
6. **Log requests** - Display method and path for debugging

## Limitations and Workarounds

**No dynamic route table**: WFL doesn't have dictionaries/maps, so routes must be checked with if/otherwise chains. For many routes, use a consistent pattern to keep code maintainable.

**No regex matching**: Use `starts with`, `ends with`, and `contains` for pattern matching.

**Module scope**: Route modules can read parent scope variables but create their own scope for new variables.

## Example Project Structure

```
my_wfl_server/
├── server.wfl                 # Main server with routing logic
├── routes/
│   ├── home.wfl              # GET / handler
│   ├── about.wfl             # GET /about handler
│   ├── api_status.wfl        # GET /api/status handler
│   ├── api_users.wfl         # GET /api/users handler
│   ├── api_create_user.wfl   # POST /api/users handler
│   └── user_profile.wfl      # GET /users/:id handler
└── public/
    ├── style.css
    ├── app.js
    └── images/
```

## Summary

WFL's module loading feature enables clean, modular routing by:
1. Checking request path and method
2. Setting REQUEST_* context variables
3. Loading the appropriate .wfl module
4. Letting the module handle the response

This pattern scales well for small to medium applications and keeps route handling code organized and maintainable.
