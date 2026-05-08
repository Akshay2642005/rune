use time::{Date, Month};

pub struct DateUtils;

impl DateUtils {
    /// Parse a date from a URL string (format: YYYY-MM-DD)
    pub fn parse_from_url(s: Option<String>) -> Option<Date> {
        let s = s?;
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return None;
        }
        let year: i32 = parts[0].parse().ok()?;
        let month: u8 = parts[1].parse().ok()?;
        let day: u8 = parts[2].parse().ok()?;
        let month = Month::try_from(month).ok()?;
        Date::from_calendar_date(year, month, day).ok()
    }

    /// Get the previous month and year
    pub fn prev_month_year(month: Month, year: i32) -> (Month, i32) {
        if month == Month::January {
            (Month::December, year - 1)
        } else {
            (month.previous(), year)
        }
    }

    /// Get the next month and year
    pub fn next_month_year(month: Month, year: i32) -> (Month, i32) {
        if month == Month::December {
            (Month::January, year + 1)
        } else {
            (month.next(), year)
        }
    }
}
