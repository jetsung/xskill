use colored::Colorize;
use regex::Regex;

/// Empty value placeholder displayed as dimmed " - "
const EMPTY_PLACEHOLDER: &str = " - ";

/// Print a table with colored header row and aligned columns.
///
/// - Headers are printed cyan + bold.
/// - Empty values (`""`) are replaced with `" - "` (dimmed).
/// - Columns are padded to the maximum width of header or data.
///
/// # Example
/// ```text
/// NAME   TYPE URL
/// antfu  git  https://github.com/antfu/skills
/// ```
pub fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if headers.is_empty() {
        return;
    }

    let num_cols = headers.len();

    // Calculate column widths (max of header len vs data len)
    let mut col_widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, val) in row.iter().enumerate() {
            if i < num_cols {
                let display_len = if val.is_empty() { EMPTY_PLACEHOLDER.len() } else { visible_length(val) };
                if display_len > col_widths[i] {
                    col_widths[i] = display_len;
                }
            }
        }
    }

    // Print header row
    for (i, h) in headers.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", pad_right(h, col_widths[i]).cyan().bold());
    }
    println!();

    // Print data rows
    for row in rows {
        for i in 0..num_cols {
            if i > 0 {
                print!(" ");
            }
            let val = row.get(i).map(|s| s.as_str()).unwrap_or("");
            if val.is_empty() {
                print!("{}", pad_right(EMPTY_PLACEHOLDER, col_widths[i]).dimmed());
            } else {
                print!("{}", pad_right(val, col_widths[i]));
            }
        }
        println!();
    }
}

/// Calculate visible length of a string, ignoring ANSI escape sequences.
fn visible_length(s: &str) -> usize {
    let re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").chars().count()
}

/// Right-pad `s` with spaces to reach `width`.
fn pad_right(s: &str, width: usize) -> String {
    let len = visible_length(s);
    if len >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pad_right_exact() {
        assert_eq!(pad_right("abc", 3), "abc");
    }

    #[test]
    fn test_pad_right_shorter() {
        assert_eq!(pad_right("ab", 5), "ab   ");
    }

    #[test]
    fn test_pad_right_longer() {
        assert_eq!(pad_right("abcdef", 3), "abcdef");
    }

    #[test]
    fn test_empty_placeholder() {
        // Empty values should be replaced with " - "
        let headers = vec!["NAME", "URL"];
        let rows = vec![
            vec!["antfu".to_string(), "https://example.com".to_string()],
            vec!["other".to_string(), String::new()],
        ];
        // Just verify it doesn't panic
        print_table(&headers, &rows);
    }
}
