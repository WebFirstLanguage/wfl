# WFL Async/Await Patterns Guide

> **📖 Async Documentation Navigation**
> - **You are here:** Practical async patterns and examples for everyday use
> - **For complete spec:** See [WFL Async Reference](../wfldocs/WFL-async.md) - Full language specification
> - **For I/O operations:** See [WFL I/O Reference](../wfldocs/WFL-io.md) - File and network I/O

## Overview

WFL supports asynchronous programming through natural language syntax that makes concurrent operations easy to understand and write. The `wait for` keyword is used to handle asynchronous operations, allowing programs to perform non-blocking I/O operations like web requests and file operations.

## Basic Async Syntax

### The `wait for` Statement

The fundamental async pattern in WFL uses the `wait for` statement:

```wfl
wait for [async operation] as [result variable]
```

This syntax:
- Pauses execution until the async operation completes
- Stores the result in the specified variable
- Handles errors gracefully
- Maintains readable, natural language flow

## Web Requests

### Basic HTTP GET Request

```wfl
// Simple web request
wait for open url at "https://www.google.com" and read content as response
display "Response received: " with response

// Store response for later use
wait for open url at "https://api.example.com/data" and read content as api_data
display "API returned: " with api_data
```

### Handling Different Response Types

```wfl
// Text content
wait for open url at "https://httpbin.org/user-agent" and read content as user_agent_info
display "User agent response: " with user_agent_info

// JSON API responses (would need JSON parsing functions)
wait for open url at "https://api.github.com/users/octocat" and read content as github_user
display "GitHub user data: " with github_user

// HTML content processing
wait for open url at "https://example.com" and read content as html_content
check if contains of html_content and "<title>":
    display "Page has a title tag"
otherwise:
    display "No title tag found"
end
```

### Multiple Web Requests

```wfl
// Sequential requests
action fetch_multiple_apis:
    store results as []
    
    store urls as [
        "https://httpbin.org/uuid",
        "https://httpbin.org/ip", 
        "https://httpbin.org/user-agent"
    ]
    
    count url in urls:
        display "Fetching: " with url
        wait for open url at url and read content as response
        push of results and response
        display "  Response length: " with length of response
    end
    
    return results
end

store all_responses as fetch_multiple_apis
display "Fetched " with length of all_responses with " responses"
```

### Error Handling with Web Requests

```wfl
// Robust error handling
action safe_web_request with url:
    try:
        display "Requesting: " with url
        wait for open url at url and read content as response
        
        check if length of response > 0:
            display "Success: Received " with length of response with " characters"
            return response
        otherwise:
            display "Warning: Empty response from " with url
            return nothing
        end
        
    when network error:
        display "Network error connecting to: " with url
        return nothing
        
    when timeout:
        display "Timeout accessing: " with url
        return nothing
        
    otherwise:
        display "Unknown error accessing: " with url
        return nothing
    end try
end

// Usage
store response as safe_web_request of "https://httpbin.org/delay/2"
check if isnothing of response:
    display "Request failed"
otherwise:
    display "Request succeeded"
end
```

## File I/O Operations

### Asynchronous File Reading

```wfl
// Read file asynchronously  
wait for open file at "data.txt" and read content as file_content
display "File contains: " with file_content

// Read multiple files
action read_multiple_files with filenames:
    store file_contents as []
    
    count filename in filenames:
        display "Reading: " with filename
        wait for open file at filename and read content as content
        
        store file_info as [filename, content]
        push of file_contents and file_info
    end
    
    return file_contents
end

store files_to_read as ["config.txt", "data.txt", "log.txt"]
store all_file_data as read_multiple_files of files_to_read

count file_data in all_file_data:
    store filename as file_data[0]
    store content as file_data[1]
    display filename with " contains " with length of content with " characters"
end
```

### Asynchronous File Writing

```wfl
// Write file asynchronously
store report_data as "System Status: All systems operational\nTimestamp: " with current_date
wait for create file at "status_report.txt" with report_data as write_result
display "Report written successfully"

// Write multiple files
action write_log_files with log_entries:
    count entry in log_entries:
        store timestamp as entry[0]
        store level as entry[1] 
        store message as entry[2]
        
        store filename as "logs/" with timestamp with "_" with level with ".log"
        store log_content as "[" with timestamp with "] " with level with ": " with message
        
        display "Writing log: " with filename
        wait for create file at filename with log_content as result
    end
    
    display "All log files written"
end

// Usage
store log_data as [
    ["2025-08-04", "INFO", "Application started"],
    ["2025-08-04", "DEBUG", "Configuration loaded"],
    ["2025-08-04", "ERROR", "Database connection failed"]
]

write_log_files of log_data
```

### File Operations with Error Handling

```wfl
// Safe file operations
action safe_file_read with filename:
    try:
        check if path_exists of filename:
            wait for open file at filename and read content as content
            display "Successfully read: " with filename
            return content
        otherwise:
            display "File not found: " with filename
            return nothing
        end
        
    when permission denied:
        display "Permission denied reading: " with filename
        return nothing
        
    when file locked:
        display "File is locked: " with filename
        return nothing
        
    otherwise:
        display "Unknown error reading: " with filename
        return nothing
    end try
end

// Batch file processing
action process_file_batch with filenames:
    store successful_reads as 0
    store failed_reads as 0
    
    count filename in filenames:
        store content as safe_file_read of filename
        
        check if isnothing of content:
            store failed_reads as failed_reads + 1
        otherwise:
            store successful_reads as successful_reads + 1
            // Process the content here
            display "Processing " with length of content with " characters from " with filename
        end
    end
    
    display "Batch complete: " with successful_reads with " succeeded, " with failed_reads with " failed"
end
```

## Advanced Async Patterns

### Concurrent Operations (Conceptual)

```wfl
// While WFL doesn't have true parallelism, you can structure
// async operations efficiently

action fetch_and_process_data:
    // Start multiple async operations
    display "Starting data collection..."
    
    // These would run sequentially but with async I/O
    wait for open url at "https://api.weather.com/current" and read content as weather_data
    wait for open file at "local_data.txt" and read content as local_data
    wait for open url at "https://api.news.com/headlines" and read content as news_data
    
    // Process all data together
    store combined_data as "Weather: " with weather_data with "\n" 
    store combined_data as combined_data with "Local: " with local_data with "\n"
    store combined_data as combined_data with "News: " with news_data
    
    wait for create file at "daily_report.txt" with combined_data as write_result
    display "Daily report compiled and saved"
    
    return combined_data
end
```

### Timeout and Retry Patterns

```wfl
// Retry mechanism for unreliable operations
action retry_web_request with url and max_attempts:
    store attempts as 0
    
    while attempts < max_attempts:
        store attempts as attempts + 1
        display "Attempt " with attempts with " of " with max_attempts
        
        try:
            wait for open url at url and read content as response
            display "Success on attempt " with attempts
            return response
            
        when timeout:
            display "Timeout on attempt " with attempts
            check if attempts < max_attempts:
                display "Retrying in 2 seconds..."
                // Would need a delay function
            end
            
        when network error:
            display "Network error on attempt " with attempts
            check if attempts < max_attempts:
                display "Retrying..."
            end
            
        otherwise:
            display "Unknown error on attempt " with attempts
        end try
    end
    
    display "All attempts failed"
    return nothing
end

// Usage
store reliable_response as retry_web_request of "https://unreliable-api.com/data" and 3
check if isnothing of reliable_response:
    display "Could not fetch data after multiple attempts"
otherwise:
    display "Successfully fetched data"
end
```

### Async Data Pipeline

```wfl
// Create an async data processing pipeline
action create_data_pipeline with input_urls and output_file:
    store pipeline_results as []
    
    display "Starting data pipeline with " with length of input_urls with " sources"
    
    // Stage 1: Fetch all data
    count url in input_urls:
        display "Fetching from: " with url
        wait for open url at url and read content as raw_data
        
        // Stage 2: Process each piece of data
        store processed_data as process_raw_data of raw_data
        push of pipeline_results and processed_data
        
        display "Processed data from " with url with " (" with length of processed_data with " chars)"
    end
    
    // Stage 3: Combine all results
    store final_output as ""
    count result in pipeline_results:
        store final_output as final_output with result with "\n---\n"
    end
    
    // Stage 4: Save final result
    display "Saving pipeline results to: " with output_file
    wait for create file at output_file with final_output as save_result
    
    display "Pipeline complete: " with length of pipeline_results with " sources processed"
    return final_output
end

action process_raw_data with raw_data:
    // Simple processing - extract first 100 characters and add timestamp
    store preview as substring of raw_data and 0 and 100
    store timestamp as current_date
    return "Processed on " with timestamp with ":\n" with preview with "..."
end

// Usage
store data_sources as [
    "https://httpbin.org/uuid",
    "https://httpbin.org/ip",
    "https://httpbin.org/user-agent"
]

store pipeline_output as create_data_pipeline of data_sources and "pipeline_results.txt"
```

## File System Async Operations

### Directory Operations

```wfl
// Async directory processing
action process_directory_async with directory_path:
    display "Processing directory: " with directory_path
    
    // List directory contents
    store all_files as list_dir of directory_path
    store file_contents as []
    
    count filename in all_files:
        store file_path as path_join of directory_path and filename
        
        check if is_file of file_path:
            display "Reading file: " with filename
            wait for open file at file_path and read content as content
            
            store file_info as [filename, length of content, content]
            push of file_contents and file_info
        end
    end
    
    // Generate summary report
    store total_files as length of file_contents
    store total_chars as 0
    
    count file_info in file_contents:
        store char_count as file_info[1]
        store total_chars as total_chars + char_count
    end
    
    store summary as "Directory Summary:\n"
    store summary as summary with "Total files: " with total_files with "\n"
    store summary as summary with "Total characters: " with total_chars with "\n\n"
    
    count file_info in file_contents:
        store filename as file_info[0]
        store char_count as file_info[1]
        store summary as summary with filename with ": " with char_count with " chars\n"
    end
    
    // Save summary
    store summary_file as path_join of directory_path and "directory_summary.txt"
    wait for create file at summary_file with summary as write_result
    
    display "Directory processing complete. Summary saved to: " with summary_file
    return file_contents
end
```

### Log File Processing

```wfl
// Process log files asynchronously
action process_log_files with log_directory:
    display "Processing log files in: " with log_directory
    
    // Find all log files
    store log_files as glob of "*.log" and log_directory
    store error_count as 0
    store warning_count as 0
    store total_lines as 0
    
    count log_file in log_files:
        display "Analyzing: " with log_file
        wait for open file at log_file and read content as log_content
        
        // Simple line counting (would need better string functions)
        store content_length as length of log_content
        store estimated_lines as content_length / 50  // Rough estimate
        store total_lines as total_lines + estimated_lines
        
        // Count errors and warnings
        check if contains of log_content and "ERROR":
            store error_count as error_count + 1
        end
        check if contains of log_content and "WARNING":
            store warning_count as warning_count + 1
        end
    end
    
    // Generate report
    store report as "Log Analysis Report\n"
    store report as report with "===================\n"
    store report as report with "Files processed: " with length of log_files with "\n"
    store report as report with "Total lines (estimated): " with total_lines with "\n"
    store report as report with "Files with errors: " with error_count with "\n"
    store report as report with "Files with warnings: " with warning_count with "\n"
    
    store report_file as path_join of log_directory and "log_analysis_report.txt"
    wait for create file at report_file with report as write_result
    
    display "Log analysis complete. Report saved to: " with report_file
    return [error_count, warning_count, total_lines]
end
```

## Error Handling Strategies

### Comprehensive Error Handling

```wfl
// Robust async operation with comprehensive error handling
action robust_async_operation with operation_type and resource_path:
    try:
        check if operation_type is "web":
            wait for open url at resource_path and read content as result
            
        check if operation_type is "file":
            wait for open file at resource_path and read content as result
            
        otherwise:
            display "Unknown operation type: " with operation_type
            return nothing
        end
        
        // Validate result
        check if isnothing of result:
            display "Operation returned nothing"
            return nothing
        end
        
        check if length of result is 0:
            display "Operation returned empty result"
            return nothing
        end
        
        display "Operation successful: " with length of result with " characters retrieved"
        return result
        
    when network error:
        display "Network error accessing: " with resource_path
        return nothing
        
    when file not found:
        display "Resource not found: " with resource_path
        return nothing
        
    when permission denied:
        display "Permission denied accessing: " with resource_path
        return nothing
        
    when timeout:
        display "Timeout accessing: " with resource_path
        return nothing
        
    otherwise:
        display "Unknown error accessing: " with resource_path
        return nothing
    end try
end
```

### Graceful Degradation

```wfl
// System that gracefully handles failures
action create_resilient_system:
    store primary_data as nothing
    store fallback_data as nothing
    store cached_data as nothing
    
    // Try primary source
    display "Attempting primary data source..."
    store primary_data as robust_async_operation of "web" and "https://api.primary.com/data"
    
    check if isnothing of primary_data:
        display "Primary source failed, trying fallback..."
        store fallback_data as robust_async_operation of "web" and "https://api.fallback.com/data"
        
        check if isnothing of fallback_data:
            display "Fallback failed, using cached data..."
            store cached_data as robust_async_operation of "file" and "cache/last_known_data.txt"
            
            check if isnothing of cached_data:
                display "All data sources failed"
                return "No data available"
            otherwise:
                display "Using cached data"
                return cached_data
            end
        otherwise:
            display "Using fallback data"
            return fallback_data
        end
    otherwise:
        display "Using primary data"
        // Cache the successful result
        wait for create file at "cache/last_known_data.txt" with primary_data as cache_result
        return primary_data
    end
end
```

## Best Practices

### 1. Always Handle Errors

```wfl
// Good: Proper error handling
action good_async_example with url:
    try:
        wait for open url at url and read content as response
        return response
    when network error:
        display "Network issue with: " with url
        return nothing
    otherwise:
        display "Unexpected error with: " with url
        return nothing
    end try
end

// Bad: No error handling
action bad_async_example with url:
    wait for open url at url and read content as response
    return response  // Will crash if request fails
end
```

### 2. Validate Results

```wfl
// Always validate async results
action validated_request with url:
    wait for open url at url and read content as response
    
    check if isnothing of response:
        display "Request returned nothing"
        return nothing
    end
    
    check if length of response < 10:
        display "Response seems too short, might be an error"
        return nothing
    end
    
    check if contains of response and "error":
        display "Response contains error message"
        return nothing
    end
    
    return response
end
```

### 3. Provide User Feedback

```wfl
// Keep users informed during long operations
action user_friendly_async with urls:
    store total_urls as length of urls
    store completed as 0
    store results as []
    
    display "Starting to process " with total_urls with " URLs..."
    
    count url in urls:
        store completed as completed + 1
        display "Processing " with completed with "/" with total_urls with ": " with url
        
        wait for open url at url and read content as response
        push of results and response
        
        store percentage as round of (completed / total_urls * 100)
        display "Progress: " with percentage with "% complete"
    end
    
    display "All URLs processed successfully!"
    return results
end
```

### 4. Use Meaningful Variable Names

```wfl
// Good: Clear variable names
wait for open url at "https://api.weather.com/current" and read content as weather_response
wait for open file at "user_preferences.json" and read content as user_settings

// Bad: Unclear variable names  
wait for open url at "https://api.weather.com/current" and read content as data
wait for open file at "user_preferences.json" and read content as stuff
```

## Integration with Other Modules

### With Text Module

```wfl
// Process web content with text functions
action analyze_web_content with url:
    wait for open url at url and read content as html_content
    
    store content_length as length of html_content
    store uppercase_content as touppercase of html_content
    
    check if contains of html_content and "<title>":
        display "Page has title tag"
    end
    
    check if contains of html_content and "<!DOCTYPE html>":
        display "Valid HTML document"
    end
    
    display "Content analysis:"
    display "- Length: " with content_length with " characters"
    display "- Contains HTML: " with contains of html_content and "<html>"
    
    return html_content
end
```

### With Time Module

```wfl
// Timestamped async operations
action timestamped_fetch with url:
    store start_time as datetime_now
    display "Starting fetch at: " with format_datetime of start_time and "%H:%M:%S"
    
    wait for open url at url and read content as response
    
    store end_time as datetime_now
    display "Completed fetch at: " with format_datetime of end_time and "%H:%M:%S"
    
    // Would need time arithmetic for duration
    display "Fetch completed successfully"
    return response
end
```

## See Also

- [Core Module](core-module.md) - Basic utilities for async result handling
- [Text Module](text-module.md) - Processing async text content
- [Filesystem Module](filesystem-module.md) - File I/O operations
- [WFL Error Handling](../language-reference/wfl-errors.md) - Error handling patterns
- [WFL Language Reference](../language-reference/wfl-spec.md) - Complete language specification