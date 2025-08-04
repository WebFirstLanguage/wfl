# WFL Cookbook

This cookbook provides ready-to-use recipes for common programming tasks in WFL. Each recipe is self-contained and can be copied, pasted, and adapted for your needs.

## Table of Contents

### [Text Processing](#text-processing)
- [Clean and validate user input](#clean-and-validate-user-input)
- [Extract email addresses from text](#extract-email-addresses-from-text)
- [Generate slugs from titles](#generate-slugs-from-titles)
- [Format names properly](#format-names-properly)
- [Word count and text statistics](#word-count-and-text-statistics)

### [File Operations](#file-operations)
- [Read configuration files](#read-configuration-files)
- [Process CSV files](#process-csv-files)
- [Backup files with timestamps](#backup-files-with-timestamps)
- [Monitor file changes](#monitor-file-changes)
- [Batch rename files](#batch-rename-files)

### [Web and API](#web-and-api)
- [Make HTTP requests with error handling](#make-http-requests-with-error-handling)
- [Download and save files](#download-and-save-files)
- [Parse JSON responses](#parse-json-responses)
- [Web scraping basics](#web-scraping-basics)
- [Rate-limited API calls](#rate-limited-api-calls)

### [Data Processing](#data-processing)
- [Sort and filter lists](#sort-and-filter-lists)
- [Group data by criteria](#group-data-by-criteria)
- [Calculate statistics](#calculate-statistics)
- [Remove duplicates](#remove-duplicates)
- [Merge and join datasets](#merge-and-join-datasets)

### [Date and Time](#date-and-time)
- [Format dates for display](#format-dates-for-display)
- [Calculate time differences](#calculate-time-differences)
- [Parse various date formats](#parse-various-date-formats)
- [Work with timezones](#work-with-timezones)
- [Schedule recurring tasks](#schedule-recurring-tasks)

### [Math and Calculations](#math-and-calculations)
- [Financial calculations](#financial-calculations)
- [Statistical functions](#statistical-functions)
- [Unit conversions](#unit-conversions)
- [Random data generation](#random-data-generation)
- [Geometric calculations](#geometric-calculations)

### [System and Utilities](#system-and-utilities)
- [Environment configuration](#environment-configuration)
- [Logging and debugging](#logging-and-debugging)
- [Performance timing](#performance-timing)
- [Memory usage monitoring](#memory-usage-monitoring)
- [Error reporting](#error-reporting)

---

## Text Processing

### Clean and validate user input

```wfl
define action called clean user input:
    parameter raw input as Text
    
    // Remove extra whitespace
    store cleaned as trim of raw input
    
    // Remove special characters if needed
    store allowed chars as "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 .-_"
    store result as ""
    
    for each char in cleaned:
        check if contains of allowed chars and char:
            store result as result with char
        end check
    end for
    
    // Limit length
    check if length of result is greater than 100:
        store result as substring of result and 0 and 100
    end check
    
    return result
end action

// Usage
store user name as clean user input with "  John@#$%Doe!!!  "
display "Cleaned name: '" with user name with "'"  // "John Doe"
```

### Extract email addresses from text

```wfl
define action called extract emails:
    parameter text as Text
    
    store emails as []
    store words as split text by " "
    
    for each word in words:
        // Simple email check - contains @ and a dot after @
        check if contains of word and "@":
            store at position as indexof of word and "@"
            store after at as substring of word and (at position plus 1) and (length of word minus at position minus 1)
            
            check if contains of after at and ".":
                add word to emails
            end check
        end check
    end for
    
    return emails
end action

// Usage
store text as "Contact us at info@example.com or support@company.org for help"
store found emails as extract emails with text
display "Found emails: " with found emails
```

### Generate slugs from titles

```wfl
define action called create slug:
    parameter title as Text
    
    // Convert to lowercase
    store slug as tolowercase of title
    
    // Replace spaces and special chars with hyphens
    store slug as replace of slug and " " and "-"
    store slug as replace of slug and "!" and ""
    store slug as replace of slug and "?" and ""
    store slug as replace of slug and "," and ""
    store slug as replace of slug and "." and ""
    store slug as replace of slug and "'" and ""
    store slug as replace of slug and "\"" and ""
    
    // Remove multiple consecutive hyphens
    while contains of slug and "--":
        store slug as replace of slug and "--" and "-"
    end while
    
    // Trim hyphens from start and end
    store slug as trim of slug with "-"
    
    return slug
end action

// Usage
store title as "Hello, World! How are you?"
store slug as create slug with title
display "Slug: " with slug  // "hello-world-how-are-you"
```

### Format names properly

```wfl
define action called format name:
    parameter full name as Text
    
    store parts as split full name by " "
    store formatted parts as []
    
    for each part in parts:
        // Skip empty parts
        check if length of part is greater than 0:
            // Capitalize first letter, lowercase the rest
            store first char as touppercase of (substring of part and 0 and 1)
            store rest as tolowercase of (substring of part and 1 and (length of part minus 1))
            store formatted part as first char with rest
            add formatted part to formatted parts
        end check
    end for
    
    return join formatted parts with " "
end action

// Usage
store name as "jOHN mCdONALD"
store formatted as format name with name
display "Formatted: " with formatted  // "John Mcdonald"
```

### Word count and text statistics

```wfl
define action called analyze text:
    parameter text as Text
    
    store word count as 0
    store character count as length of text
    store line count as 1
    store paragraph count as 1
    
    store words as split text by " "
    for each word in words:
        check if length of (trim of word) is greater than 0:
            store word count as word count plus 1
        end check
    end for
    
    // Count lines
    for each char in text:
        check if char is "\n":
            store line count as line count plus 1
        end check
    end for
    
    // Count paragraphs (double newlines)
    store paragraphs as split text by "\n\n"
    store paragraph count as length of paragraphs
    
    return {
        "words": word count,
        "characters": character count,
        "lines": line count,
        "paragraphs": paragraph count,
        "average_words_per_line": word count divided by line count
    }
end action

// Usage
store sample text as "Hello world!\nThis is a test.\n\nNew paragraph here."
store stats as analyze text with sample text
display "Words: " with stats.words
display "Characters: " with stats.characters
display "Lines: " with stats.lines
display "Paragraphs: " with stats.paragraphs
```

---

## File Operations

### Read configuration files

```wfl
define action called load config:
    parameter config file as Text
    parameter defaults as Object
    
    store config as defaults
    
    try:
        check if file exists config file:
            store file content as read file config file
            store loaded config as parse json from file content
            
            // Merge with defaults
            for each key in loaded config:
                store config[key] as loaded config[key]
            end for
            
            display "Configuration loaded from " with config file
        otherwise:
            display "Config file not found, using defaults"
            
            // Save defaults to file
            store default json as stringify json from defaults
            write default json to file config file
            display "Created default config file: " with config file
        end check
    catch error:
        display "Error loading config: " with error.message
        display "Using default configuration"
    end try
    
    return config
end action

// Usage
store default settings as {
    "server_port": 8080,
    "debug_mode": false,
    "database_url": "sqlite://./app.db",
    "max_connections": 100
}

store config as load config with "app.config.json" and default settings
display "Server will run on port: " with config.server_port
```

### Process CSV files

```wfl
define action called process csv:
    parameter filename as Text
    parameter has header as Boolean
    
    store rows as []
    store headers as []
    
    try:
        store lines as read lines from filename
        store line number as 0
        
        for each line in lines:
            store columns as split line by ","
            
            // Clean up columns (remove quotes, trim whitespace)
            store cleaned columns as []
            for each column in columns:
                store cleaned as trim of column
                check if starts with cleaned and "\"" and ends with cleaned and "\"":
                    store cleaned as substring of cleaned and 1 and (length of cleaned minus 2)
                end check
                add cleaned to cleaned columns
            end for
            
            check if line number is 0 and has header is yes:
                store headers as cleaned columns
            otherwise:
                add cleaned columns to rows
            end check
            
            store line number as line number plus 1
        end for
        
        display "Processed " with length of rows with " rows from " with filename
        
        return {
            "headers": headers,
            "rows": rows,
            "count": length of rows
        }
        
    catch error:
        display "Error processing CSV: " with error.message
        return { "headers": [], "rows": [], "count": 0 }
    end try
end action

// Usage
store sales data as process csv with "sales.csv" and yes

display "Headers: " with sales data.headers

for each row in sales data.rows:
    display "Row: " with row
end for
```

### Backup files with timestamps

```wfl
define action called backup file:
    parameter original file as Text
    
    try:
        check if file exists original file:
            store now as current time
            store timestamp as format now as "YYYY-MM-DD_HH-mm-ss"
            
            // Get file extension
            store extension as ""
            store dot position as lastindexof of original file and "."
            check if dot position is not -1:
                store extension as substring of original file and dot position and (length of original file minus dot position)
            end check
            
            // Create backup filename
            store backup name as original file with "_backup_" with timestamp with extension
            
            // Copy file
            store content as read file original file
            write content to file backup name
            
            display "Backup created: " with backup name
            return backup name
        otherwise:
            display "Original file not found: " with original file
            return nothing
        end check
    catch error:
        display "Backup failed: " with error.message
        return nothing
    end try
end action

// Usage
store backup path as backup file with "important-data.json"
check if backup path is not nothing:
    display "Backup successful: " with backup path
end check
```

### Monitor file changes

```wfl
define action called monitor file:
    parameter filename as Text
    parameter check interval as Number  // seconds
    
    store last modified as nothing
    store is first check as yes
    
    while yes:
        try:
            check if file exists filename:
                store current modified as modified date of filename
                
                check if is first check is yes:
                    store last modified as current modified
                    store is first check as no
                    display "Started monitoring: " with filename
                check if current modified is not equal to last modified:
                    display "File changed: " with filename
                    display "New modification time: " with current modified
                    store last modified as current modified
                end check
            otherwise:
                display "File not found: " with filename
                wait check interval seconds
                continue
            end check
        catch error:
            display "Error monitoring file: " with error.message
        end try
        
        wait check interval seconds
    end while
end action

// Usage (run in background)
// monitor file with "config.json" and 5
```

### Batch rename files

```wfl
define action called batch rename:
    parameter directory as Text
    parameter pattern as Text
    parameter replacement as Text
    
    store renamed count as 0
    
    try:
        store files as list files in directory
        
        for each file in files:
            check if contains of file and pattern:
                store new name as replace of file and pattern and replacement
                store old path as directory with "/" with file
                store new path as directory with "/" with new name
                
                try:
                    rename file old path to new path
                    display "Renamed: " with file with " → " with new name
                    store renamed count as renamed count plus 1
                catch error:
                    display "Failed to rename " with file with ": " with error.message
                end try
            end check
        end for
        
        display "Renamed " with renamed count with " files"
        return renamed count
        
    catch error:
        display "Error accessing directory: " with error.message
        return 0
    end try
end action

// Usage
store count as batch rename with "./images" and "IMG_" and "photo_"
display "Renamed " with count with " files"
```

---

## Web and API

### Make HTTP requests with error handling

```wfl
define action called safe http request:
    parameter url as Text
    parameter method as Text
    parameter headers as Object
    parameter timeout as Number
    
    store default headers as {
        "User-Agent": "WFL-App/1.0",
        "Accept": "application/json"
    }
    
    // Merge headers
    for each key in headers:
        store default headers[key] as headers[key]
    end for
    
    try:
        store request as create http request:
            url is url
            method is method
            headers is default headers
            timeout is timeout
        end
        
        store response as await send request
        
        return {
            "success": yes,
            "status": response.status,
            "headers": response.headers,
            "body": response.body,
            "error": nothing
        }
        
    catch timeout error:
        return {
            "success": no,
            "status": 0,
            "headers": {},
            "body": "",
            "error": "Request timed out after " with timeout with " seconds"
        }
    catch network error:
        return {
            "success": no,
            "status": 0,
            "headers": {},
            "body": "",
            "error": "Network error: " with network error.message
        }
    catch error:
        return {
            "success": no,
            "status": 0,
            "headers": {},
            "body": "",
            "error": "Unknown error: " with error.message
        }
    end try
end action

// Usage
store result as safe http request with "https://api.github.com/users/octocat" and "GET" and {} and 30

check if result.success:
    display "Status: " with result.status
    store user data as parse json from result.body
    display "User: " with user data.name
otherwise:
    display "Request failed: " with result.error
end check
```

### Download and save files

```wfl
define action called download file:
    parameter url as Text
    parameter local path as Text
    parameter show progress as Boolean
    
    try:
        display "Starting download: " with url
        
        store response as await web.get(url)
        
        check if response.status is 200:
            store file size as length of response.body
            
            check if show progress:
                display "Downloaded " with file size with " bytes"
            end check
            
            write response.body to file local path
            display "File saved: " with local path
            
            return {
                "success": yes,
                "size": file size,
                "path": local path
            }
        otherwise:
            return {
                "success": no,
                "error": "HTTP " with response.status
            }
        end check
        
    catch error:
        return {
            "success": no,
            "error": error.message
        }
    end try
end action

// Usage
store download result as download file with "https://httpbin.org/json" and "./downloaded.json" and yes

check if download result.success:
    display "Download complete: " with download result.size with " bytes"
otherwise:
    display "Download failed: " with download result.error
end check
```

### Parse JSON responses

```wfl
define action called fetch json data:
    parameter api url as Text
    parameter expected fields as List
    
    try:
        store response as await web.get(api url)
        
        check if response.status is not 200:
            return {
                "success": no,
                "error": "HTTP " with response.status,
                "data": nothing
            }
        end check
        
        store json data as parse json from response.body
        
        // Validate expected fields are present
        store missing fields as []
        for each field in expected fields:
            check if json data[field] is undefined:
                add field to missing fields
            end check
        end for
        
        check if length of missing fields is greater than 0:
            return {
                "success": no,
                "error": "Missing fields: " with (join missing fields with ", "),
                "data": json data
            }
        end check
        
        return {
            "success": yes,
            "error": nothing,
            "data": json data
        }
        
    catch parse error:
        return {
            "success": no,
            "error": "Invalid JSON: " with parse error.message,
            "data": nothing
        }
    catch error:
        return {
            "success": no,
            "error": error.message,
            "data": nothing
        }
    end try
end action

// Usage
store expected as ["login", "name", "public_repos"]
store result as fetch json data with "https://api.github.com/users/octocat" and expected

check if result.success:
    store user as result.data
    display "User: " with user.name with " (" with user.login with ")"
    display "Repositories: " with user.public_repos
otherwise:
    display "API Error: " with result.error
end check
```

### Web scraping basics

```wfl
define action called scrape webpage:
    parameter url as Text
    parameter selectors as Object
    
    try:
        store response as await web.get(url)
        
        check if response.status is not 200:
            return { "success": no, "error": "HTTP " with response.status }
        end check
        
        store html as response.body
        store extracted data as {}
        
        // Simple text extraction (in a real implementation, you'd use proper HTML parsing)
        for each key in selectors:
            store selector as selectors[key]
            
            // For demonstration - extract text between tags
            check if selector starts with "title":
                store start tag as "<title>"
                store end tag as "</title>"
                store start pos as indexof of html and start tag
                store end pos as indexof of html and end tag
                
                check if start pos is not -1 and end pos is not -1:
                    store title as substring of html and (start pos plus length of start tag) and (end pos minus start pos minus length of start tag)
                    store extracted data[key] as trim of title
                end check
            end check
        end for
        
        return {
            "success": yes,
            "data": extracted data,
            "url": url
        }
        
    catch error:
        return {
            "success": no,
            "error": error.message
        }
    end try
end action

// Usage
store selectors as { "title": "title" }
store result as scrape webpage with "https://example.com" and selectors

check if result.success:
    display "Page title: " with result.data.title
otherwise:
    display "Scraping failed: " with result.error
end check
```

### Rate-limited API calls

```wfl
define action called rate limited request:
    parameter urls as List
    parameter delay seconds as Number
    parameter max retries as Number
    
    store results as []
    store request count as 0
    
    for each url in urls:
        store request count as request count plus 1
        store retry count as 0
        store success as no
        
        while retry count is less than max retries and success is no:
            try:
                display "Request " with request count with "/" with length of urls with ": " with url
                
                store response as await web.get(url)
                
                check if response.status is 429:  // Rate limited
                    display "Rate limited, waiting..."
                    wait (delay seconds times 2) seconds
                    store retry count as retry count plus 1
                    continue
                end check
                
                add {
                    "url": url,
                    "status": response.status,
                    "success": response.status is 200,
                    "body": response.body
                } to results
                
                store success as yes
                
            catch error:
                display "Error: " with error.message
                store retry count as retry count plus 1
                
                check if retry count is less than max retries:
                    wait delay seconds seconds
                end check
            end try
        end while
        
        check if success is no:
            add {
                "url": url,
                "status": 0,
                "success": no,
                "body": "Failed after " with max retries with " retries"
            } to results
        end check
        
        // Rate limiting delay between requests
        check if request count is less than length of urls:
            wait delay seconds seconds
        end check
    end for
    
    return results
end action

// Usage
store api urls as [
    "https://httpbin.org/delay/1",
    "https://httpbin.org/status/200",
    "https://httpbin.org/status/404"
]

store results as rate limited request with api urls and 2 and 3

for each result in results:
    display result.url with " -> " with result.status with " (" with result.success with ")"
end for
```

---

## Data Processing

### Sort and filter lists

```wfl
define action called sort list:
    parameter items as List
    parameter sort key as Text
    parameter ascending as Boolean
    
    // Simple bubble sort implementation
    store sorted items as copy of items
    store n as length of sorted items
    
    count from 0 to (n minus 2):
        count from 0 to (n minus count minus 2) as j:
            store current as sorted items at j
            store next as sorted items at (j plus 1)
            
            store current value as current
            store next value as next
            
            // Get sort key value if specified
            check if sort key is not "":
                store current value as current[sort key]
                store next value as next[sort key]
            end check
            
            store should swap as no
            check if ascending is yes:
                store should swap as current value is greater than next value
            otherwise:
                store should swap as current value is less than next value
            end check
            
            check if should swap:
                store sorted items[j] as next
                store sorted items[j plus 1] as current
            end check
        end count
    end count
    
    return sorted items
end action

define action called filter list:
    parameter items as List
    parameter filter function as Function
    
    store filtered as []
    
    for each item in items:
        check if filter function(item):
            add item to filtered
        end check
    end for
    
    return filtered
end action

// Usage
store people as [
    { "name": "Alice", "age": 30, "salary": 50000 },
    { "name": "Bob", "age": 25, "salary": 45000 },
    { "name": "Charlie", "age": 35, "salary": 60000 }
]

// Sort by age
store sorted by age as sort list with people and "age" and yes
display "Sorted by age:"
for each person in sorted by age:
    display person.name with " (age " with person.age with ")"
end for

// Filter high earners
store high earners as filter list with people and (lambda item: item.salary > 50000)
display "High earners:"
for each person in high earners:
    display person.name with " - $" with person.salary
end for
```

### Group data by criteria

```wfl
define action called group by:
    parameter items as List
    parameter group key as Text
    
    store groups as {}
    
    for each item in items:
        store key value as item[group key]
        
        check if groups[key value] is undefined:
            store groups[key value] as []
        end check
        
        add item to groups[key value]
    end for
    
    return groups
end action

// Usage
store sales as [
    { "product": "Laptop", "category": "Electronics", "amount": 1200 },
    { "product": "Mouse", "category": "Electronics", "amount": 25 },
    { "product": "Desk", "category": "Furniture", "amount": 300 },
    { "product": "Chair", "category": "Furniture", "amount": 150 },
    { "product": "Phone", "category": "Electronics", "amount": 800 }
]

store grouped as group by with sales and "category"

for each category in grouped:
    display "Category: " with category
    store total as 0
    
    for each item in grouped[category]:
        display "  " with item.product with " - $" with item.amount
        store total as total plus item.amount
    end for
    
    display "  Total: $" with total
    display ""
end for
```

### Calculate statistics

```wfl
define action called calculate stats:
    parameter numbers as List
    
    check if length of numbers is 0:
        return { "error": "Empty list" }
    end check
    
    store sum as 0
    store min value as numbers at 0
    store max value as numbers at 0
    
    for each number in numbers:
        store sum as sum plus number
        
        check if number is less than min value:
            store min value as number
        end check
        
        check if number is greater than max value:
            store max value as number
        end check
    end for
    
    store count as length of numbers
    store mean as sum divided by count
    
    // Calculate median
    store sorted numbers as sort list with numbers and "" and yes
    store median as 0
    store middle as count divided by 2
    
    check if count mod 2 is 0:
        store median as ((sorted numbers at (middle minus 1)) plus (sorted numbers at middle)) divided by 2
    otherwise:
        store median as sorted numbers at floor of middle
    end check
    
    // Calculate standard deviation
    store variance sum as 0
    for each number in numbers:
        store diff as number minus mean
        store variance sum as variance sum plus (diff times diff)
    end for
    
    store variance as variance sum divided by count
    store std dev as sqrt of variance
    
    return {
        "count": count,
        "sum": sum,
        "mean": mean,
        "median": median,
        "min": min value,
        "max": max value,
        "range": max value minus min value,
        "variance": variance,
        "std_dev": std dev
    }
end action

// Usage
store test scores as [85, 92, 78, 96, 88, 91, 83, 89, 94, 87]
store stats as calculate stats with test scores

display "Test Score Statistics:"
display "Count: " with stats.count
display "Mean: " with round of stats.mean
display "Median: " with stats.median
display "Min: " with stats.min
display "Max: " with stats.max
display "Range: " with stats.range
display "Standard Deviation: " with round of stats.std_dev
```

### Remove duplicates

```wfl
define action called remove duplicates:
    parameter items as List
    parameter key as Text  // Optional: field to compare for objects
    
    store unique items as []
    store seen values as []
    
    for each item in items:
        store comparison value as item
        
        check if key is not "":
            store comparison value as item[key]
        end check
        
        check if not contains of seen values and comparison value:
            add item to unique items
            add comparison value to seen values
        end check
    end for
    
    return unique items
end action

// Usage with simple values
store numbers as [1, 2, 3, 2, 4, 1, 5, 3]
store unique numbers as remove duplicates with numbers and ""
display "Original: " with numbers
display "Unique: " with unique numbers

// Usage with objects
store users as [
    { "id": 1, "name": "Alice" },
    { "id": 2, "name": "Bob" },
    { "id": 1, "name": "Alice" },
    { "id": 3, "name": "Charlie" }
]

store unique users as remove duplicates with users and "id"
display "Unique users:"
for each user in unique users:
    display user.id with ": " with user.name
end for
```

### Merge and join datasets

```wfl
define action called join datasets:
    parameter left dataset as List
    parameter right dataset as List
    parameter left key as Text
    parameter right key as Text
    parameter join type as Text  // "inner", "left", "right", "outer"
    
    store result as []
    
    check if join type is "inner" or join type is "left":
        for each left item in left dataset:
            store left value as left item[left key]
            store found match as no
            
            for each right item in right dataset:
                store right value as right item[right key]
                
                check if left value is right value:
                    // Merge items
                    store merged as copy of left item
                    for each key in right item:
                        check if key is not right key:  // Don't duplicate join key
                            store merged[key] as right item[key]
                        end check
                    end for
                    
                    add merged to result
                    store found match as yes
                end check
            end for
            
            // For left join, include unmatched left items
            check if join type is "left" and found match is no:
                add left item to result
            end check
        end for
    end check
    
    return result
end action

// Usage
store customers as [
    { "id": 1, "name": "Alice", "city": "New York" },
    { "id": 2, "name": "Bob", "city": "Los Angeles" },
    { "id": 3, "name": "Charlie", "city": "Chicago" }
]

store orders as [
    { "customer_id": 1, "product": "Laptop", "amount": 1200 },
    { "customer_id": 2, "product": "Phone", "amount": 800 },
    { "customer_id": 1, "product": "Mouse", "amount": 25 }
]

store customer orders as join datasets with customers and orders and "id" and "customer_id" and "inner"

display "Customer Orders:"
for each order in customer orders:
    display order.name with " from " with order.city with " ordered " with order.product with " for $" with order.amount
end for
```

---

## Date and Time

### Format dates for display

```wfl
define action called format date:
    parameter date as Date
    parameter format string as Text
    
    store year as format date as "YYYY"
    store month as format date as "MM"
    store day as format date as "DD"
    store hour as format date as "HH"
    store minute as format date as "mm"
    store second as format date as "ss"
    
    store formatted as format string
    store formatted as replace of formatted and "YYYY" and year
    store formatted as replace of formatted and "MM" and month
    store formatted as replace of formatted and "DD" and day
    store formatted as replace of formatted and "HH" and hour
    store formatted as replace of formatted and "mm" and minute
    store formatted as replace of formatted and "ss" and second
    
    return formatted
end action

// Usage
store now as current time
display "ISO format: " with format date with now and "YYYY-MM-DD HH:mm:ss"
display "US format: " with format date with now and "MM/DD/YYYY"
display "European format: " with format date with now and "DD.MM.YYYY"
display "Time only: " with format date with now and "HH:mm"
```

### Calculate time differences

```wfl
define action called time difference:
    parameter start time as Date
    parameter end time as Date
    parameter unit as Text
    
    store diff milliseconds as end time minus start time
    store diff seconds as diff milliseconds divided by 1000
    store diff minutes as diff seconds divided by 60
    store diff hours as diff minutes divided by 60
    store diff days as diff hours divided by 24
    
    check if unit is "seconds":
        return diff seconds
    check if unit is "minutes":
        return diff minutes
    check if unit is "hours":
        return diff hours
    check if unit is "days":
        return diff days
    otherwise:
        return diff milliseconds
    end check
end action

define action called format duration:
    parameter milliseconds as Number
    
    store days as floor of (milliseconds divided by (1000 times 60 times 60 times 24))
    store remaining as milliseconds mod (1000 times 60 times 60 times 24)
    
    store hours as floor of (remaining divided by (1000 times 60 times 60))
    store remaining as remaining mod (1000 times 60 times 60)
    
    store minutes as floor of (remaining divided by (1000 times 60))
    store seconds as floor of ((remaining mod (1000 times 60)) divided by 1000)
    
    store parts as []
    
    check if days is greater than 0:
        add (days with " days") to parts
    end check
    
    check if hours is greater than 0:
        add (hours with " hours") to parts
    end check
    
    check if minutes is greater than 0:
        add (minutes with " minutes") to parts
    end check
    
    check if seconds is greater than 0:
        add (seconds with " seconds") to parts
    end check
    
    check if length of parts is 0:
        return "0 seconds"
    end check
    
    return join parts with ", "
end action

// Usage
store start as parse date from "2024-01-01 09:00:00"
store end as parse date from "2024-01-03 15:30:00"

store diff as time difference with start and end and "milliseconds"
store formatted as format duration with diff

display "Time difference: " with formatted
display "Hours: " with time difference with start and end and "hours"
display "Days: " with time difference with start and end and "days"
```

### Parse various date formats

```wfl
define action called parse flexible date:
    parameter date string as Text
    
    store date string as trim of date string
    
    // Try different formats
    store formats as [
        "YYYY-MM-DD HH:mm:ss",
        "YYYY-MM-DD",
        "MM/DD/YYYY",
        "DD/MM/YYYY",
        "MM-DD-YYYY",
        "DD-MM-YYYY",
        "YYYY/MM/DD"
    ]
    
    for each format in formats:
        try:
            store parsed as parse date from date string with format
            return {
                "success": yes,
                "date": parsed,
                "format": format
            }
        catch error:
            // Try next format
            continue
        end try
    end for
    
    return {
        "success": no,
        "error": "Could not parse date: " with date string
    }
end action

// Usage
store test dates as [
    "2024-12-25",
    "12/25/2024",
    "25/12/2024",
    "2024/12/25 14:30:00",
    "Dec 25, 2024"
]

for each date string in test dates:
    store result as parse flexible date with date string
    
    check if result.success:
        display date string with " -> " with result.date with " (format: " with result.format with ")"
    otherwise:
        display date string with " -> " with result.error
    end check
end for
```

### Work with timezones

```wfl
define action called convert timezone:
    parameter date as Date
    parameter from timezone as Text
    parameter to timezone as Text
    
    // This is a simplified example - real timezone conversion is more complex
    store timezone offsets as {
        "UTC": 0,
        "EST": -5,
        "PST": -8,
        "GMT": 0,
        "JST": 9,
        "CET": 1
    }
    
    store from offset as timezone offsets[from timezone] or 0
    store to offset as timezone offsets[to timezone] or 0
    store hour difference as to offset minus from offset
    
    store converted as date plus (hour difference times 60 times 60 times 1000)
    
    return {
        "date": converted,
        "from": from timezone,
        "to": to timezone,
        "offset_hours": hour difference
    }
end action

// Usage
store utc time as parse date from "2024-12-25 12:00:00"
store est result as convert timezone with utc time and "UTC" and "EST"
store jst result as convert timezone with utc time and "UTC" and "JST"

display "UTC: " with format date with utc time and "YYYY-MM-DD HH:mm:ss"
display "EST: " with format date with est result.date and "YYYY-MM-DD HH:mm:ss"
display "JST: " with format date with jst result.date and "YYYY-MM-DD HH:mm:ss"
```

### Schedule recurring tasks

```wfl
define action called create schedule:
    parameter task name as Text
    parameter interval type as Text  // "daily", "weekly", "monthly"
    parameter interval value as Number
    parameter start time as Date
    
    return {
        "name": task name,
        "type": interval type,
        "interval": interval value,
        "start": start time,
        "next_run": start time
    }
end action

define action called get next run time:
    parameter schedule as Object
    
    store next as schedule.next_run
    
    check if schedule.type is "daily":
        store next as next plus (schedule.interval times 24 times 60 times 60 times 1000)
    check if schedule.type is "weekly":
        store next as next plus (schedule.interval times 7 times 24 times 60 times 60 times 1000)
    check if schedule.type is "monthly":
        // Simplified - add 30 days per month
        store next as next plus (schedule.interval times 30 times 24 times 60 times 60 times 1000)
    end check
    
    return next
end action

// Usage
store backup schedule as create schedule with "Database Backup" and "daily" and 1 and (current time)
store report schedule as create schedule with "Weekly Report" and "weekly" and 1 and (current time)

display "Backup next run: " with format date with backup schedule.next_run and "YYYY-MM-DD HH:mm:ss"
display "Report next run: " with format date with report schedule.next_run and "YYYY-MM-DD HH:mm:ss"

// Update next run time
store backup schedule.next_run as get next run time with backup schedule
display "Backup next run updated: " with format date with backup schedule.next_run and "YYYY-MM-DD HH:mm:ss"
```

---

## Math and Calculations

### Financial calculations

```wfl
define action called calculate compound interest:
    parameter principal as Number
    parameter annual rate as Number
    parameter compounds per year as Number
    parameter years as Number
    
    store rate per period as annual rate divided by compounds per year
    store total periods as compounds per year times years
    store final amount as principal times ((1 plus rate per period) to the power of total periods)
    store interest earned as final amount minus principal
    
    return {
        "principal": principal,
        "final_amount": final amount,
        "interest_earned": interest earned,
        "effective_rate": (interest earned divided by principal) times 100
    }
end action

define action called calculate loan payment:
    parameter loan amount as Number
    parameter annual rate as Number
    parameter years as Number
    
    store monthly rate as annual rate divided by 12
    store num payments as years times 12
    
    check if monthly rate is 0:
        store monthly payment as loan amount divided by num payments
    otherwise:
        store monthly payment as loan amount times (monthly rate times ((1 plus monthly rate) to the power of num payments)) divided by (((1 plus monthly rate) to the power of num payments) minus 1)
    end check
    
    store total paid as monthly payment times num payments
    store total interest as total paid minus loan amount
    
    return {
        "loan_amount": loan amount,
        "monthly_payment": monthly payment,
        "total_paid": total paid,
        "total_interest": total interest
    }
end action

// Usage
store investment as calculate compound interest with 10000 and 0.07 and 12 and 10
display "Investment Results:"
display "Principal: $" with investment.principal
display "Final Amount: $" with round of investment.final_amount
display "Interest Earned: $" with round of investment.interest_earned
display "Effective Rate: " with round of investment.effective_rate with "%"

display ""

store loan as calculate loan payment with 250000 and 0.04 and 30
display "Loan Payment Results:"
display "Loan Amount: $" with loan.loan_amount
display "Monthly Payment: $" with round of loan.monthly_payment
display "Total Paid: $" with round of loan.total_paid
display "Total Interest: $" with round of loan.total_interest
```

### Statistical functions

```wfl
define action called linear regression:
    parameter x values as List
    parameter y values as List
    
    store n as length of x values
    store sum x as 0
    store sum y as 0
    store sum xy as 0
    store sum x squared as 0
    
    for each i from 0 to (n minus 1):
        store x as x values at i
        store y as y values at i
        
        store sum x as sum x plus x
        store sum y as sum y plus y
        store sum xy as sum xy plus (x times y)
        store sum x squared as sum x squared plus (x times x)
    end for
    
    store slope as ((n times sum xy) minus (sum x times sum y)) divided by ((n times sum x squared) minus (sum x times sum x))
    store intercept as (sum y minus (slope times sum x)) divided by n
    
    // Calculate R-squared
    store mean y as sum y divided by n
    store ss tot as 0
    store ss res as 0
    
    for each i from 0 to (n minus 1):
        store x as x values at i
        store y as y values at i
        store predicted y as (slope times x) plus intercept
        
        store ss tot as ss tot plus ((y minus mean y) times (y minus mean y))
        store ss res as ss res plus ((y minus predicted y) times (y minus predicted y))
    end for
    
    store r squared as 1 minus (ss res divided by ss tot)
    
    return {
        "slope": slope,
        "intercept": intercept,
        "r_squared": r squared,
        "equation": "y = " with slope with "x + " with intercept
    }
end action

// Usage
store x data as [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
store y data as [2.1, 3.9, 6.2, 7.8, 10.1, 12.2, 13.8, 16.1, 18.0, 20.2]

store regression as linear regression with x data and y data

display "Linear Regression Results:"
display "Equation: " with regression.equation
display "R-squared: " with round of regression.r_squared
display "Slope: " with round of regression.slope
display "Intercept: " with round of regression.intercept
```

### Unit conversions

```wfl
define action called create converter:
    parameter unit type as Text
    
    store conversions as {}
    
    check if unit type is "length":
        store conversions as {
            "mm_to_cm": 0.1,
            "cm_to_m": 0.01,
            "m_to_km": 0.001,
            "in_to_ft": 1/12,
            "ft_to_yd": 1/3,
            "yd_to_mi": 1/1760,
            "in_to_cm": 2.54,
            "ft_to_m": 0.3048,
            "mi_to_km": 1.609344
        }
    check if unit type is "weight":
        store conversions as {
            "g_to_kg": 0.001,
            "kg_to_ton": 0.001,
            "oz_to_lb": 1/16,
            "lb_to_kg": 0.453592,
            "kg_to_lb": 2.20462
        }
    check if unit type is "temperature":
        // Special handling needed for temperature
        store conversions as {
            "c_to_f": "celsius * 9/5 + 32",
            "f_to_c": "(fahrenheit - 32) * 5/9",
            "c_to_k": "celsius + 273.15",
            "k_to_c": "kelvin - 273.15"
        }
    end check
    
    return conversions
end action

define action called convert units:
    parameter value as Number
    parameter from unit as Text
    parameter to unit as Text
    parameter unit type as Text
    
    store converter as create converter with unit type
    store conversion key as from unit with "_to_" with to unit
    
    check if unit type is "temperature":
        check if conversion key is "c_to_f":
            return (value times 9 divided by 5) plus 32
        check if conversion key is "f_to_c":
            return (value minus 32) times 5 divided by 9
        check if conversion key is "c_to_k":
            return value plus 273.15
        check if conversion key is "k_to_c":
            return value minus 273.15
        end check
    otherwise:
        store factor as converter[conversion key]
        check if factor is not undefined:
            return value times factor
        otherwise:
            return "Conversion not supported: " with from unit with " to " with to unit
        end check
    end check
end action

// Usage
store length result as convert units with 100 and "cm" and "m" and "length"
display "100 cm = " with length result with " m"

store weight result as convert units with 5 and "lb" and "kg" and "weight"
display "5 lb = " with round of weight result with " kg"

store temp result as convert units with 25 and "c" and "f" and "temperature"
display "25°C = " with temp result with "°F"
```

### Random data generation

```wfl
define action called random integer:
    parameter min as Number
    parameter max as Number
    
    return floor of ((random times (max minus min plus 1)) plus min)
end action

define action called random string:
    parameter length as Number
    parameter character set as Text
    
    store default chars as "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
    store chars as character set or default chars
    store result as ""
    
    count from 1 to length:
        store random index as random integer with 0 and (length of chars minus 1)
        store random char as substring of chars and random index and 1
        store result as result with random char
    end count
    
    return result
end action

define action called generate test data:
    parameter count as Number
    
    store first names as ["Alice", "Bob", "Charlie", "Diana", "Eve", "Frank", "Grace", "Henry"]
    store last names as ["Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis"]
    store domains as ["example.com", "test.org", "demo.net", "sample.co"]
    
    store test data as []
    
    count from 1 to count:
        store first name as first names at (random integer with 0 and (length of first names minus 1))
        store last name as last names at (random integer with 0 and (length of last names minus 1))
        store domain as domains at (random integer with 0 and (length of domains minus 1))
        
        store person as {
            "id": count,
            "first_name": first name,
            "last_name": last name,
            "full_name": first name with " " with last name,
            "email": tolowercase of first name with "." with tolowercase of last name with "@" with domain,
            "age": random integer with 18 and 65,
            "salary": random integer with 30000 and 100000,
            "department": ["Engineering", "Sales", "Marketing", "HR"] at (random integer with 0 and 3)
        }
        
        add person to test data
    end count
    
    return test data
end action

// Usage
store test people as generate test data with 5

display "Generated Test Data:"
for each person in test people:
    display person.full_name with " (" with person.age with ") - " with person.email
    display "  Department: " with person.department with ", Salary: $" with person.salary
    display ""
end for

store random code as random string with 8 and "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
display "Random code: " with random code
```

### Geometric calculations

```wfl
define action called calculate circle:
    parameter radius as Number
    
    store pi as 3.14159265359
    store area as pi times radius times radius
    store circumference as 2 times pi times radius
    store diameter as 2 times radius
    
    return {
        "radius": radius,
        "diameter": diameter,
        "area": area,
        "circumference": circumference
    }
end action

define action called calculate triangle:
    parameter base as Number
    parameter height as Number
    parameter side a as Number
    parameter side b as Number
    parameter side c as Number
    
    store area as (base times height) divided by 2
    store perimeter as side a plus side b plus side c
    
    // Heron's formula for area if all sides provided
    check if side a is greater than 0 and side b is greater than 0 and side c is greater than 0:
        store s as perimeter divided by 2
        store heron area as sqrt of (s times (s minus side a) times (s minus side b) times (s minus side c))
        store area as heron area
    end check
    
    return {
        "base": base,
        "height": height,
        "area": area,
        "perimeter": perimeter,
        "side_a": side a,
        "side_b": side b,
        "side_c": side c
    }
end action

define action called distance between points:
    parameter x1 as Number
    parameter y1 as Number
    parameter x2 as Number
    parameter y2 as Number
    
    store dx as x2 minus x1
    store dy as y2 minus y1
    store distance as sqrt of ((dx times dx) plus (dy times dy))
    
    return {
        "point1": [x1, y1],
        "point2": [x2, y2],
        "distance": distance,
        "dx": dx,
        "dy": dy
    }
end action

// Usage
store circle as calculate circle with 5
display "Circle (radius " with circle.radius with "):"
display "  Area: " with round of circle.area
display "  Circumference: " with round of circle.circumference

display ""

store triangle as calculate triangle with 10 and 8 and 6 and 8 and 10
display "Triangle:"
display "  Area: " with triangle.area
display "  Perimeter: " with triangle.perimeter

display ""

store distance as distance between points with 0 and 0 and 3 and 4
display "Distance from (0,0) to (3,4): " with distance.distance
```

---

## System and Utilities

### Environment configuration

```wfl
define action called load environment:
    parameter env file as Text
    parameter defaults as Object
    
    store environment as copy of defaults
    
    try:
        check if file exists env file:
            store lines as read lines from env file
            
            for each line in lines:
                store trimmed as trim of line
                
                // Skip comments and empty lines
                check if length of trimmed is 0 or starts with trimmed and "#":
                    continue
                end check
                
                // Parse KEY=VALUE
                store equals pos as indexof of trimmed and "="
                check if equals pos is not -1:
                    store key as trim of (substring of trimmed and 0 and equals pos)
                    store value as trim of (substring of trimmed and (equals pos plus 1) and (length of trimmed minus equals pos minus 1))
                    
                    // Remove quotes if present
                    check if starts with value and "\"" and ends with value and "\"":
                        store value as substring of value and 1 and (length of value minus 2)
                    end check
                    
                    store environment[key] as value
                end check
            end for
            
            display "Environment loaded from " with env file
        otherwise:
            display "Environment file not found: " with env file
        end check
    catch error:
        display "Error loading environment: " with error.message
    end try
    
    return environment
end action

// Usage
store default env as {
    "NODE_ENV": "development",
    "PORT": "3000",
    "DATABASE_URL": "sqlite://./app.db",
    "DEBUG": "false"
}

store env as load environment with ".env" and default env

display "Environment Configuration:"
for each key in env:
    display key with "=" with env[key]
end for
```

### Logging and debugging

```wfl
define action called create logger:
    parameter log level as Text
    parameter log file as Text
    
    store log levels as {
        "DEBUG": 0,
        "INFO": 1,
        "WARN": 2,
        "ERROR": 3
    }
    
    store current level as log levels[log level] or 1
    
    return {
        "level": current level,
        "file": log file,
        "levels": log levels
    }
end action

define action called log message:
    parameter logger as Object
    parameter level as Text
    parameter message as Text
    parameter data as Object
    
    store message level as logger.levels[level] or 1
    
    // Only log if message level is >= current level
    check if message level is greater than or equal to logger.level:
        store timestamp as format current time as "YYYY-MM-DD HH:mm:ss"
        store log entry as "[" with timestamp with "] " with level with ": " with message
        
        check if data is not nothing:
            store log entry as log entry with " | Data: " with stringify json from data
        end check
        
        display log entry
        
        // Write to file if specified
        check if logger.file is not "":
            try:
                append log entry with "\n" to file logger.file
            catch error:
                display "Failed to write to log file: " with error.message
            end try
        end check
    end check
end action

// Usage
store logger as create logger with "INFO" and "app.log"

log message with logger and "INFO" and "Application started" and { "version": "1.0.0" }
log message with logger and "DEBUG" and "This won't show" and nothing
log message with logger and "WARN" and "Low disk space" and { "available": "2GB" }
log message with logger and "ERROR" and "Database connection failed" and { "host": "localhost", "port": 5432 }
```

### Performance timing

```wfl
define action called create timer:
    parameter name as Text
    
    return {
        "name": name,
        "start_time": current time,
        "end_time": nothing,
        "duration": nothing
    }
end action

define action called stop timer:
    parameter timer as Object
    
    store timer.end_time as current time
    store timer.duration as timer.end_time minus timer.start_time
    
    return timer
end action

define action called benchmark function:
    parameter function to test as Function
    parameter iterations as Number
    parameter name as Text
    
    store total time as 0
    store results as []
    
    display "Benchmarking " with name with " (" with iterations with " iterations)..."
    
    count from 1 to iterations:
        store timer as create timer with name
        
        // Run the function
        store result as function to test()
        
        store timer as stop timer with timer
        store total time as total time plus timer.duration
        
        add {
            "iteration": count,
            "duration": timer.duration,
            "result": result
        } to results
    end count
    
    store average time as total time divided by iterations
    store min time as minimum of (map results to item.duration)
    store max time as maximum of (map results to item.duration)
    
    display "Benchmark Results for " with name with ":"
    display "  Total time: " with total time with "ms"
    display "  Average time: " with average time with "ms"
    display "  Min time: " with min time with "ms"
    display "  Max time: " with max time with "ms"
    
    return {
        "name": name,
        "iterations": iterations,
        "total_time": total time,
        "average_time": average time,
        "min_time": min time,
        "max_time": max time,
        "results": results
    }
end action

// Usage
define action called test function:
    store sum as 0
    count from 1 to 1000:
        store sum as sum plus count
    end count
    return sum
end action

store timer as create timer with "Main process"

// Do some work
store benchmark as benchmark function with test function and 10 and "Sum calculation"

store timer as stop timer with timer
display "Total execution time: " with timer.duration with "ms"
```

### Memory usage monitoring

```wfl
define action called get memory usage:
    // This would interface with system APIs in a real implementation
    return {
        "used": 45.2,      // MB
        "free": 954.8,     // MB
        "total": 1000.0,   // MB
        "usage_percent": 4.52
    }
end action

define action called memory monitor:
    parameter check interval as Number
    parameter alert threshold as Number
    
    store monitoring as yes
    store alert count as 0
    
    while monitoring:
        store memory as get memory usage
        
        display "Memory usage: " with memory.usage_percent with "% (" with memory.used with "MB used)"
        
        check if memory.usage_percent is greater than alert threshold:
            store alert count as alert count plus 1
            display "⚠️  Memory usage alert! " with memory.usage_percent with "% exceeds threshold of " with alert threshold with "%"
            
            check if alert count is greater than 3:
                display "🚨 Critical: Multiple memory alerts. Consider investigating."
            end check
        otherwise:
            store alert count as 0  // Reset counter when usage drops
        end check
        
        wait check interval seconds
    end while
end action

// Usage
store current memory as get memory usage
display "Current memory usage:"
display "  Used: " with current memory.used with "MB"
display "  Free: " with current memory.free with "MB"
display "  Usage: " with current memory.usage_percent with "%"

// Start monitoring (uncomment to run)
// memory monitor with 10 and 80
```

### Error reporting

```wfl
define action called create error reporter:
    parameter service name as Text
    parameter environment as Text
    parameter api endpoint as Text
    
    return {
        "service": service name,
        "environment": environment,
        "endpoint": api endpoint,
        "error_count": 0
    }
end action

define action called report error:
    parameter reporter as Object
    parameter error as Object
    parameter context as Object
    
    store reporter.error_count as reporter.error_count plus 1
    
    store error report as {
        "service": reporter.service,
        "environment": reporter.environment,
        "timestamp": current time,
        "error": {
            "message": error.message,
            "type": error.type or "Unknown",
            "stack": error.stack or ""
        },
        "context": context,
        "error_id": random string with 8 and "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    }
    
    // Log locally
    display "🚨 Error Report (" with error report.error_id with "):"
    display "  Service: " with error report.service
    display "  Message: " with error report.error.message
    display "  Context: " with stringify json from error report.context
    
    // Send to external service (simplified)
    try:
        store json data as stringify json from error report
        store response as await web.post(reporter.endpoint, json data)
        
        check if response.status is 200:
            display "✅ Error reported successfully"
        otherwise:
            display "❌ Failed to report error: HTTP " with response.status
        end check
    catch network error:
        display "❌ Network error reporting to external service"
        
        // Fallback: save to local file
        store filename as "errors_" with format current time as "YYYY-MM-DD" with ".log"
        append stringify json from error report with "\n" to file filename
        display "💾 Error saved to local file: " with filename
    end try
end action

// Usage
store reporter as create error reporter with "MyApp" and "production" and "https://errors.example.com/api/report"

try:
    store result as 10 divided by 0
catch math error:
    report error with reporter and math error and {
        "operation": "division",
        "operands": [10, 0],
        "user_id": "user123"
    }
end try
```

---

This cookbook provides practical, ready-to-use solutions for common programming tasks in WFL. Each recipe can be adapted and modified to fit your specific needs. The examples demonstrate WFL's natural language syntax while solving real-world problems efficiently.

Remember to test these recipes in your specific environment and modify them as needed for your use case!