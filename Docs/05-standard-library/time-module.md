# Time Module

The Time module provides date and time operations. Work with dates, times, formatting, and time calculations.

## Current Time Functions

### now

**Purpose:** Get the current time of day.

**Signature:**
```wfl
now
```

**Returns:** Time - Current time

**Example:**
```wfl
store current_time as now
display "Current time: " with current_time
```

---

### today

**Purpose:** Get the current date.

**Signature:**
```wfl
today
```

**Returns:** Date - Current date

**Example:**
```wfl
store current_date as today
display "Today is: " with current_date
```

---

### datetime_now

**Purpose:** Get current date and time combined.

**Signature:**
```wfl
datetime_now
```

**Returns:** DateTime - Current date and time

**Example:**
```wfl
store current as datetime_now
display "Current datetime: " with current
```

---

### current time in milliseconds

**Purpose:** Get current timestamp in milliseconds since epoch.

**Signature:**
```wfl
current time in milliseconds
```

**Returns:** Number - Unix timestamp in milliseconds

**Example:**
```wfl
store timestamp as current time in milliseconds
display "Timestamp: " with timestamp

// Useful for timing operations
store start as current time in milliseconds
// ... do work ...
store end as current time in milliseconds
store elapsed as end minus start
display "Elapsed: " with elapsed with "ms"
```

---

## Date Creation

### create_date

**Purpose:** Create a date from year, month, day.

**Signature:**
```wfl
create_date of <year> and <month> and <day>
```

**Parameters:**
- `year` (Number): Year (e.g., 2026)
- `month` (Number): Month (1-12)
- `day` (Number): Day of month (1-31)

**Returns:** Date value

**Example:**
```wfl
store birthday as create_date of 1995 and 3 and 15
display "Birthday: " with birthday

store new_year as create_date of 2026 and 1 and 1
display "New Year: " with new_year
```

---

### create_time

**Purpose:** Create a time from hours, minutes, seconds.

**Signature:**
```wfl
create_time of <hours> and <minutes> and <seconds>
```

**Parameters:**
- `hours` (Number): Hour (0-23)
- `minutes` (Number): Minute (0-59)
- `seconds` (Number): Second (0-59)

**Returns:** Time value

**Example:**
```wfl
store morning as create_time of 8 and 30 and 0
display "Morning: " with morning

store afternoon as create_time of 14 and 45 and 30
display "Afternoon: " with afternoon
```

---

## Date Components

### year / month / day

**Purpose:** Extract components from a date.

**Signature:**
```wfl
year of <date>
month of <date>
day of <date>
```

**Returns:** Number - The component value

**Example:**
```wfl
store test_date as create_date of 2025 and 8 and 9

display "Year: " with year of test_date       // 2025
display "Month: " with month of test_date     // 8
display "Day: " with day of test_date         // 9
```

---

### dayofweek

**Purpose:** Get day of week from a date.

**Signature:**
```wfl
dayofweek of <date>
```

**Returns:** Number - Day of week (0=Sunday, 6=Saturday)

**Example:**
```wfl
store date as create_date of 2026 and 1 and 9
store dow as dayofweek of date
display "Day of week: " with dow
```

---

## Time Components

### hour / minute / second

**Purpose:** Extract components from a time.

**Signature:**
```wfl
hour of <time>
minute of <time>
second of <time>
```

**Returns:** Number - The component value

**Example:**
```wfl
store time as create_time of 14 and 45 and 30

display "Hour: " with hour of time        // 14
display "Minute: " with minute of time    // 45
display "Second: " with second of time    // 30
```

---

## Date Math

### add_days

**Purpose:** Add days to a date.

**Signature:**
```wfl
add_days of <date> and <days>
```

**Parameters:**
- `date` (Date): Starting date
- `days` (Number): Days to add

**Returns:** Date - New date

**Example:**
```wfl
store today_date as create_date of 2026 and 1 and 9
store next_week as add_days of today_date and 7
display "One week later: " with next_week

store tomorrow as add_days of today_date and 1
display "Tomorrow: " with tomorrow
```

---

### subtract_days

**Purpose:** Subtract days from a date.

**Signature:**
```wfl
subtract_days of <date> and <days>
```

**Parameters:**
- `date` (Date): Starting date
- `days` (Number): Days to subtract

**Returns:** Date - New date

**Example:**
```wfl
store today_date as create_date of 2026 and 1 and 9
store last_week as subtract_days of today_date and 7
display "One week ago: " with last_week
```

---

### days_between

**Purpose:** Calculate days between two dates.

**Signature:**
```wfl
days_between of <date1> and <date2>
```

**Parameters:**
- `date1` (Date): First date
- `date2` (Date): Second date

**Returns:** Number - Days between (can be negative)

**Example:**
```wfl
store start as create_date of 2026 and 1 and 1
store end as create_date of 2026 and 1 and 31

store diff as days_between of start and end
display "Days in January: " with diff
// Output: Days in January: 30
```

---

## Formatting

### format_date

**Purpose:** Format a date as a string.

**Signature:**
```wfl
format_date of <date> and <format_string>
```

**Parameters:**
- `date` (Date): Date to format
- `format_string` (Text): Format pattern

**Returns:** Text - Formatted date string

**Format patterns:**
- `YYYY` - 4-digit year
- `MM` - 2-digit month
- `DD` - 2-digit day

**Example:**
```wfl
store date as create_date of 2026 and 1 and 9

display format_date of date and "YYYY-MM-DD"
// Output: 2026-01-09

display format_date of date and "MM/DD/YYYY"
// Output: 01/09/2026
```

---

### format_time

**Purpose:** Format a time as a string.

**Signature:**
```wfl
format_time of <time> and <format_string>
```

**Parameters:**
- `time` (Time): Time to format
- `format_string` (Text): Format pattern

**Returns:** Text - Formatted time string

**Format patterns:**
- `HH` - 2-digit hour (24-hour)
- `mm` - 2-digit minute
- `ss` - 2-digit second

**Example:**
```wfl
store time as create_time of 14 and 30 and 45

display format_time of time and "HH:mm:ss"
// Output: 14:30:45

display format_time of time and "HH:mm"
// Output: 14:30
```

---

### format_datetime

**Purpose:** Format a datetime as a string.

**Signature:**
```wfl
format_datetime of <datetime> and <format_string>
```

**Parameters:**
- `datetime` (DateTime): DateTime to format
- `format_string` (Text): Format pattern

**Returns:** Text - Formatted datetime string

**Example:**
```wfl
store dt as create_datetime of 2026 and 1 and 9 and 14 and 30 and 0

display format_datetime of dt and "YYYY-MM-DD HH:mm:ss"
// Output: 2026-01-09 14:30:00
```

---

## Complete Example

```wfl
display "=== Time Module Demo ==="
display ""

// Current time
display "Right now:"
display "  Date: " with today
display "  Time: " with now
display "  DateTime: " with datetime_now
display "  Timestamp: " with current time in milliseconds
display ""

// Date creation
store birthday as create_date of 1995 and 3 and 15
display "Birthday: " with birthday

store components as "Components:"
display "  Year: " with year of birthday
display "  Month: " with month of birthday
display "  Day: " with day of birthday
display ""

// Date math
store today_date as today
store next_week as add_days of today_date and 7
store last_month as subtract_days of today_date and 30

display "Dates:"
display "  Today: " with today_date
display "  Next week: " with next_week
display "  Last month: " with last_month
display ""

// Time calculations
store start_date as create_date of 2026 and 1 and 1
store end_date as create_date of 2026 and 12 and 31
store days_in_year as days_between of start_date and end_date

display "Days in 2026: " with days_in_year
display ""

display "=== Demo Complete ==="
```

## Common Patterns

### Age Calculation

```wfl
define action called calculate age with parameters birth_year:
    store today_date as today
    store current_year as year of today_date
    store age as current_year minus birth_year
    return age
end action

store age as calculate age with 1995
display "Age: " with age
```

### Days Until Event

```wfl
store today_date as today
store christmas as create_date of 2026 and 12 and 25
store days_until as days_between of today_date and christmas

display "Days until Christmas: " with days_until
```

### Timestamp Logging

```wfl
define action called log with timestamp with parameters message:
    store ts as current time in milliseconds
    display ts with ": " with message
end action

call log with timestamp with "Application started"
call log with timestamp with "Processing complete"
```

## Best Practices

✅ **Use current time in milliseconds for timestamps:** Precision and comparisons

✅ **Format dates for display:** Users prefer readable dates

✅ **Store dates, not strings:** Use Date values for calculations

✅ **Validate date components:** Check month (1-12), day (1-31)

❌ **Don't compare date strings:** Use Date values

❌ **Don't hardcode dates:** Use create_date for flexibility

## What You've Learned

✅ **Current time functions** - now, today, datetime_now
✅ **Date/time creation** - create_date, create_time
✅ **Component extraction** - year, month, day, hour, minute, second
✅ **Date math** - add_days, subtract_days, days_between
✅ **Formatting** - format_date, format_time, format_datetime
✅ **Common patterns** - Age calculation, countdowns, logging

## Next Steps

**[Random Module →](random-module.md)**
Random number generation for games and simulations.

Or return to:
**[Standard Library Index →](index.md)**

---

**Previous:** [← Filesystem Module](filesystem-module.md) | **Next:** [Random Module →](random-module.md)
