# WFL Time Module API Reference

## Overview

The Time module provides comprehensive date and time functionality for WFL programs. It supports creating, formatting, parsing, and manipulating dates and times with full timezone awareness.

## Data Types

WFL has three main time-related data types:

- **Date**: Represents a calendar date (year, month, day)
- **Time**: Represents a time of day (hour, minute, second)
- **DateTime**: Represents both date and time together

## Current Date and Time Functions

### `today()`

Returns the current date.

**Parameters:** None

**Returns:** Date (current date)

**Examples:**

```wfl
// Get current date
store current_date as today
display "Today is: " with current_date

// Use in calculations
store tomorrow as add_days of today and 1
display "Tomorrow will be: " with tomorrow

// Store for later use
store program_start_date as today
// ... later in program ...
store days_running as days_between of program_start_date and today
display "Program has been running for " with days_running with " days"
```

**Natural Language Variants:**
```wfl
// All equivalent ways to get today's date
store date1 as today
store date2 as current date
store date3 as todays date
store date4 as what date is today
```

---

### `now()`

Returns the current time.

**Parameters:** None

**Returns:** Time (current time)

**Examples:**

```wfl
// Get current time
store current_time as now
display "The time is: " with current_time

// Time-based decisions
store lunch_time as create_time of 12 and 0  // 12:00 PM
store current_time as now
// Note: Time comparison would need additional functions

// Log timestamps
store start_time as now
// ... do some work ...
store end_time as now
display "Started at: " with start_time
display "Finished at: " with end_time
```

**Natural Language Variants:**
```wfl
// All equivalent ways to get current time
store time1 as now
store time2 as current time
store time3 as what time is it
store time4 as time now
```

---

### `datetime_now()`

Returns the current date and time together.

**Parameters:** None

**Returns:** DateTime (current date and time)

**Examples:**

```wfl
// Get current datetime
store right_now as datetime_now
display "Current date and time: " with right_now

// Precise timestamping
store log_entry as ["User logged in", datetime_now]
display "Log: " with log_entry

// Event scheduling
store event_created as datetime_now
display "Event created at: " with event_created
```

**Natural Language Variants:**
```wfl
// All equivalent ways to get current datetime
store dt1 as datetime_now
store dt2 as current datetime
store dt3 as date and time now
store dt4 as timestamp now
```

---

### `current_date()`

Returns the current date as a formatted string (YYYY-MM-DD).

**Parameters:** None

**Returns:** Text (formatted date string)

**Examples:**

```wfl
// Get formatted date string
store date_string as current_date
display "Date: " with date_string  // e.g., "2025-08-04"

// File naming
store filename as "backup_" with current_date with ".txt"
display "Creating file: " with filename  // "backup_2025-08-04.txt"

// Simple date comparisons
store today_string as current_date
store target_date as "2025-12-25"
display "Today: " with today_string
display "Christmas: " with target_date
```

## Formatting Functions

### `format_date(date, format)`

Formats a date according to a format string.

**Parameters:**
- `date` (Date): The date to format
- `format` (Text): Format specification string

**Returns:** Text (formatted date)

**Format Specifiers:**
- `%Y` - 4-digit year (e.g., 2025)
- `%y` - 2-digit year (e.g., 25)
- `%m` - Month as number (01-12)
- `%B` - Full month name (e.g., January)
- `%b` - Abbreviated month name (e.g., Jan)
- `%d` - Day of month (01-31)
- `%A` - Full weekday name (e.g., Monday)
- `%a` - Abbreviated weekday name (e.g., Mon)

**Examples:**

```wfl
// Different date formats
store birthday as create_date of 1990 and 12 and 25

store iso_format as format_date of birthday and "%Y-%m-%d"
display "ISO format: " with iso_format  // "1990-12-25"

store us_format as format_date of birthday and "%m/%d/%Y"
display "US format: " with us_format    // "12/25/1990"

store long_format as format_date of birthday and "%B %d, %Y"
display "Long format: " with long_format  // "December 25, 1990"

store short_format as format_date of birthday and "%d %b %y"
display "Short format: " with short_format  // "25 Dec 90"

// Current date in different formats
store today_date as today
store formal as format_date of today_date and "%A, %B %d, %Y"
display "Today is: " with formal  // "Monday, August 04, 2025"
```

**Natural Language Variants:**
```wfl
// All equivalent ways to format dates
store result as format_date of date and format
store result as format date using format
store result as date formatted as format
```

---

### `format_time(time, format)`

Formats a time according to a format string.

**Parameters:**
- `time` (Time): The time to format
- `format` (Text): Format specification string

**Returns:** Text (formatted time)

**Format Specifiers:**
- `%H` - Hour in 24-hour format (00-23)
- `%I` - Hour in 12-hour format (01-12)
- `%M` - Minutes (00-59)
- `%S` - Seconds (00-59)
- `%p` - AM/PM indicator

**Examples:**

```wfl
// Different time formats
store meeting_time as create_time of 14 and 30 and 0

store military as format_time of meeting_time and "%H:%M:%S"
display "24-hour: " with military  // "14:30:00"

store standard as format_time of meeting_time and "%I:%M %p"
display "12-hour: " with standard  // "02:30 PM"

store simple as format_time of meeting_time and "%H:%M"
display "Simple: " with simple     // "14:30"

// Current time formatting
store current_time as now
store display_time as format_time of current_time and "%I:%M %p"
display "Current time: " with display_time
```

---

### `format_datetime(datetime, format)`

Formats a datetime according to a format string.

**Parameters:**
- `datetime` (DateTime): The datetime to format
- `format` (Text): Format specification string combining date and time specifiers

**Returns:** Text (formatted datetime)

**Examples:**

```wfl
// Complete datetime formatting
store event_time as datetime_now

store full_format as format_datetime of event_time and "%Y-%m-%d %H:%M:%S"
display "Full: " with full_format  // "2025-08-04 14:30:45"

store readable as format_datetime of event_time and "%B %d, %Y at %I:%M %p"
display "Readable: " with readable  // "August 04, 2025 at 02:30 PM"

store log_format as format_datetime of event_time and "[%Y-%m-%d %H:%M:%S]"
display "Log entry: " with log_format  // "[2025-08-04 14:30:45]"
```

## Parsing Functions

### `parse_date(text, format)`

Parses a date from a text string.

**Parameters:**
- `text` (Text): The date string to parse
- `format` (Text): Format specification matching the input string

**Returns:** Date (parsed date object)

**Examples:**

```wfl
// Parse different date formats
store iso_date as parse_date of "2025-12-25" and "%Y-%m-%d"
display "Parsed ISO date: " with iso_date

store us_date as parse_date of "12/25/2025" and "%m/%d/%Y"
display "Parsed US date: " with us_date

store long_date as parse_date of "December 25, 2025" and "%B %d, %Y"
display "Parsed long date: " with long_date

// Error handling (would cause runtime error if format doesn't match)
// store bad_date as parse_date of "2025-12-25" and "%m/%d/%Y"  // Error!
```

**Practical Use Cases:**

```wfl
// User input processing
action get_birthday_from_user:
    display "Enter your birthday (MM/DD/YYYY):"
    store user_input as get_input  // Hypothetical input function
    
    store birthday as parse_date of user_input and "%m/%d/%Y"
    store formatted as format_date of birthday and "%B %d, %Y"
    display "Your birthday is: " with formatted
    
    return birthday
end

// File date processing
action process_log_file with filename:
    // Extract date from filename like "log_2025-08-04.txt"
    store date_part as substring of filename and 4 and 10  // "2025-08-04"
    store log_date as parse_date of date_part and "%Y-%m-%d"
    
    display "Processing log from: " with format_date of log_date and "%B %d, %Y"
end
```

---

### `parse_time(text, format)`

Parses a time from a text string.

**Parameters:**
- `text` (Text): The time string to parse
- `format` (Text): Format specification matching the input string

**Returns:** Time (parsed time object)

**Examples:**

```wfl
// Parse different time formats
store military_time as parse_time of "14:30" and "%H:%M"
display "Parsed 24-hour: " with military_time

store standard_time as parse_time of "2:30 PM" and "%I:%M %p"
display "Parsed 12-hour: " with standard_time

store with_seconds as parse_time of "14:30:45" and "%H:%M:%S"
display "With seconds: " with with_seconds
```

## Creation Functions

### `create_date(year, month, day)`

Creates a date from year, month, and day values.

**Parameters:**
- `year` (Number): The year (e.g., 2025)
- `month` (Number): The month (1-12)
- `day` (Number): The day of month (1-31)

**Returns:** Date (created date object)

**Examples:**

```wfl
// Create specific dates
store independence_day as create_date of 2025 and 7 and 4
store new_years as create_date of 2026 and 1 and 1
store leap_day as create_date of 2024 and 2 and 29

// Display created dates
store july_4th_formatted as format_date of independence_day and "%B %d, %Y"
display "Independence Day 2025: " with july_4th_formatted

// Date validation (would cause error for invalid dates)
// store invalid as create_date of 2025 and 13 and 1  // Error: month > 12
// store invalid2 as create_date of 2025 and 2 and 30  // Error: Feb 30th doesn't exist
```

**Practical Use Cases:**

```wfl
// Generate date ranges
action create_date_range with start_year and end_year:
    store dates as []
    
    count year from start_year to end_year:
        store jan_first as create_date of year and 1 and 1
        push of dates and jan_first
    end
    
    return dates
end

// Birthday calculations
action calculate_age with birth_year and birth_month and birth_day:
    store birthday as create_date of birth_year and birth_month and birth_day
    store today_date as today
    
    // Would need additional date arithmetic for precise age calculation
    store current_year as 2025  // Simplified
    store age_estimate as current_year - birth_year
    
    return age_estimate
end
```

---

### `create_time(hour, minute, [second])`

Creates a time from hour, minute, and optional second values.

**Parameters:**
- `hour` (Number): Hour in 24-hour format (0-23)
- `minute` (Number): Minutes (0-59)
- `second` (Number, optional): Seconds (0-59), defaults to 0

**Returns:** Time (created time object)

**Examples:**

```wfl
// Create specific times
store lunch_time as create_time of 12 and 0        // 12:00:00
store dinner_time as create_time of 18 and 30      // 18:30:00
store precise_time as create_time of 14 and 30 and 45  // 14:30:45

// Display created times
store lunch_display as format_time of lunch_time and "%I:%M %p"
display "Lunch time: " with lunch_display  // "12:00 PM"

// Schedule creation
store schedule as []
push of schedule and ["Breakfast", create_time of 8 and 0]
push of schedule and ["Lunch", create_time of 12 and 0]
push of schedule and ["Dinner", create_time of 18 and 0]

count meal in schedule:
    store meal_name as meal[0]
    store meal_time as meal[1]
    store formatted_time as format_time of meal_time and "%I:%M %p"
    display meal_name with " at " with formatted_time
end
```

## Date Arithmetic Functions

### `add_days(date, days)`

Adds a number of days to a date.

**Parameters:**
- `date` (Date): The starting date
- `days` (Number): Number of days to add (can be negative to subtract)

**Returns:** Date (new date after adding days)

**Examples:**

```wfl
// Basic date arithmetic
store today_date as today
store tomorrow as add_days of today_date and 1
store yesterday as add_days of today_date and -1
store next_week as add_days of today_date and 7

display "Today: " with today_date
display "Tomorrow: " with tomorrow
display "Yesterday: " with yesterday
display "Next week: " with next_week

// Project planning
store project_start as create_date of 2025 and 8 and 1
store project_end as add_days of project_start and 30
store milestone_1 as add_days of project_start and 10
store milestone_2 as add_days of project_start and 20

display "Project timeline:"
display "Start: " with format_date of project_start and "%B %d, %Y"
display "Milestone 1: " with format_date of milestone_1 and "%B %d, %Y"
display "Milestone 2: " with format_date of milestone_2 and "%B %d, %Y"
display "End: " with format_date of project_end and "%B %d, %Y"
```

**Natural Language Variants:**
```wfl
// All equivalent ways to add days
store result as add_days of date and 5
store result as date plus 5 days
store result as 5 days after date
store result as date + 5 days
```

---

### `days_between(date1, date2)`

Calculates the number of days between two dates.

**Parameters:**
- `date1` (Date): The starting date
- `date2` (Date): The ending date

**Returns:** Number (days difference, positive if date2 is later, negative if earlier)

**Examples:**

```wfl
// Calculate days between dates
store start_date as create_date of 2025 and 1 and 1
store end_date as create_date of 2025 and 12 and 31
store days_in_year as days_between of start_date and end_date
display "Days in 2025: " with days_in_year  // 364

// Time until future event
store today_date as today
store christmas as create_date of 2025 and 12 and 25
store days_until_christmas as days_between of today_date and christmas
display "Days until Christmas: " with days_until_christmas

// Age in days
store birthday as create_date of 1990 and 12 and 25
store age_in_days as days_between of birthday and today_date
display "You are " with age_in_days with " days old"

// Project duration
store project_start as create_date of 2025 and 8 and 1
store project_actual_end as create_date of 2025 and 9 and 15
store actual_duration as days_between of project_start and project_actual_end
display "Project took " with actual_duration with " days"
```

**Practical Use Cases:**

```wfl
// Deadline tracking
action check_deadline with task_due_date:
    store today_date as today
    store days_remaining as days_between of today_date and task_due_date
    
    check if days_remaining < 0:
        store days_overdue as abs of days_remaining
        display "Task is " with days_overdue with " days overdue!"
    check if days_remaining is 0:
        display "Task is due today!"
    check if days_remaining <= 3:
        display "Task is due in " with days_remaining with " days (urgent)"
    otherwise:
        display "Task is due in " with days_remaining with " days"
    end
end

// Subscription management
action check_subscription_status with subscription_start and subscription_length:
    store today_date as today
    store subscription_end as add_days of subscription_start and subscription_length
    store days_remaining as days_between of today_date and subscription_end
    
    check if days_remaining <= 0:
        return "expired"
    check if days_remaining <= 7:
        return "expires_soon"
    otherwise:
        return "active"
    end
end
```

## Advanced Examples

### Event Scheduling System

```wfl
// Create a simple event scheduler
action create_event_scheduler:
    store events as []
    
    action add_event with name and event_date and event_time:
        store event as [name, event_date, event_time]
        push of events and event
        
        store formatted_date as format_date of event_date and "%B %d, %Y"
        store formatted_time as format_time of event_time and "%I:%M %p"
        display "Added event: " with name with " on " with formatted_date with " at " with formatted_time
    end
    
    action show_upcoming_events:
        store today_date as today
        store upcoming as []
        
        count event in events:
            store event_name as event[0]
            store event_date as event[1]
            store event_time as event[2]
            
            store days_until as days_between of today_date and event_date
            check if days_until >= 0:
                push of upcoming and event
            end
        end
        
        display "Upcoming events:"
        count event in upcoming:
            store event_name as event[0]
            store event_date as event[1]
            store event_time as event[2]
            
            store formatted_date as format_date of event_date and "%m/%d/%Y"
            store formatted_time as format_time of event_time and "%I:%M %p"
            store days_until as days_between of today_date and event_date
            
            display "- " with event_name with " (" with formatted_date with " " with formatted_time
            display "  " with days_until with " days from now)"
        end
    end
    
    return [add_event, show_upcoming_events]
end
```

### Time-based Logging

```wfl
// Create timestamped log entries
action create_logger:
    store log_entries as []
    
    action log_message with level and message:
        store timestamp as datetime_now
        store formatted_time as format_datetime of timestamp and "%Y-%m-%d %H:%M:%S"
        
        store log_entry as [formatted_time, level, message]
        push of log_entries and log_entry
        
        display "[" with formatted_time with "] " with level with ": " with message
    end
    
    action get_daily_logs with target_date:
        store target_date_str as format_date of target_date and "%Y-%m-%d"
        store daily_logs as []
        
        count entry in log_entries:
            store entry_time as entry[0]
            store entry_level as entry[1]
            store entry_message as entry[2]
            
            // Check if entry is from target date (simplified string comparison)
            check if contains of entry_time and target_date_str:
                push of daily_logs and entry
            end
        end
        
        return daily_logs
    end
    
    return [log_message, get_daily_logs]
end

// Usage example
store logger_functions as create_logger
store log_message as logger_functions[0]
store get_daily_logs as logger_functions[1]

log_message of "INFO" and "Application started"
log_message of "DEBUG" and "Loading configuration"
log_message of "ERROR" and "Failed to connect to database"
```

### Date Range Processing

```wfl
// Generate business days (Monday-Friday) in a date range
action get_business_days with start_date and end_date:
    store business_days as []
    store current_date as start_date
    
    while days_between of current_date and end_date >= 0:
        // Note: Would need day-of-week function for complete implementation
        // For now, assume all days are business days
        push of business_days and current_date
        store current_date as add_days of current_date and 1
    end
    
    return business_days
end

// Calculate working days between dates (simplified)
action count_working_days with start_date and end_date:
    store total_days as days_between of start_date and end_date
    
    // Simplified: assume 5/7 of days are working days
    store working_days as round of (total_days * 5 / 7)
    return working_days
end
```

## Error Handling

Time functions handle various error conditions:

```wfl
// Safe date creation with validation
action safe_create_date with year and month and day:
    // Basic validation
    check if month < 1 or month > 12:
        display "Invalid month: " with month
        return nothing
    end
    
    check if day < 1 or day > 31:
        display "Invalid day: " with day
        return nothing
    end
    
    // Note: create_date function will handle detailed validation
    // like February 30th, leap years, etc.
    return create_date of year and month and day
end

// Safe parsing with error handling
action safe_parse_date with date_string and format_string:
    // In a real implementation, you'd want try/catch equivalent
    // For now, assume parse_date throws errors for invalid input
    store parsed_date as parse_date of date_string and format_string
    return parsed_date
end
```

## Integration with Other Modules

### With Text Module

```wfl
// Create readable date descriptions
action describe_date with target_date:
    store today_date as today
    store days_diff as days_between of today_date and target_date
    
    check if days_diff is 0:
        return "today"
    check if days_diff is 1:
        return "tomorrow"
    check if days_diff is -1:
        return "yesterday"
    check if days_diff > 0:
        return days_diff with " days from now"
    otherwise:
        store days_ago as abs of days_diff
        return days_ago with " days ago"
    end
end
```

### With Math Module

```wfl
// Time-based calculations
action calculate_compound_interest with principal and rate and start_date and end_date:
    store days as days_between of start_date and end_date
    store years as days / 365.25
    
    store amount as principal * (1 + rate) ^ years
    return round of amount
end
```

## Best Practices

1. **Always format dates for display**: Raw date objects may not display nicely
2. **Use consistent date formats**: Pick standard formats for your application
3. **Handle timezone considerations**: Be aware of local vs UTC time
4. **Validate user input**: Always check date/time input for validity
5. **Use appropriate precision**: Don't use seconds if you only need dates

```wfl
// Example of good time handling practices
action process_user_date with user_input:
    // Validate input format first
    check if length of user_input is not 10:
        display "Date must be in YYYY-MM-DD format"
        return nothing
    end
    
    // Parse with proper error handling
    store parsed_date as parse_date of user_input and "%Y-%m-%d"
    
    // Validate date is reasonable
    store today_date as today
    store days_diff as days_between of today_date and parsed_date
    
    check if abs of days_diff > 36500:  // More than 100 years
        display "Date seems unreasonable"
        return nothing
    end
    
    // Format for consistent display
    store formatted as format_date of parsed_date and "%B %d, %Y"
    display "Processed date: " with formatted
    
    return parsed_date
end
```

## See Also

- [Core Module](core-module.md) - Basic utilities and type checking
- [Math Module](math-module.md) - Numeric operations for calculations
- [Text Module](text-module.md) - String formatting and manipulation
- [WFL Language Reference](../language-reference/wfl-spec.md) - Complete language specification