//! Number formats shared by every crate (feature 944, research R9).

/// `n` with its digits grouped by thousands: `37141` → `"37,141"`. The application's one count
/// format; it has no locale layer.
pub fn group_digits(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::group_digits;

    #[test]
    fn digits_are_grouped_by_thousands() {
        assert_eq!(group_digits(0), "0");
        assert_eq!(group_digits(999), "999");
        assert_eq!(group_digits(1000), "1,000");
        assert_eq!(group_digits(37_141), "37,141");
        assert_eq!(group_digits(1_000_000), "1,000,000");
        assert_eq!(group_digits(u64::MAX), "18,446,744,073,709,551,615");
    }
}
