use telegram_botkit::ui::formatters::*;

#[test]
fn parse_sol_to_lamports_ok() {
    assert_eq!(parse_sol_to_lamports("1"), Some(1_000_000_000));
    assert_eq!(parse_sol_to_lamports("0.000000001"), Some(1));
    assert_eq!(parse_sol_to_lamports("1,2"), Some(1_200_000_000));
}

#[test]
fn parse_sol_to_lamports_bad() {
    assert_eq!(parse_sol_to_lamports(""), None);
    assert_eq!(parse_sol_to_lamports("a.b"), None);
}

#[test]
fn parse_percent_to_bp_ok() {
    assert_eq!(parse_percent_to_bp("1"), Some(100));
    assert_eq!(parse_percent_to_bp("1.23"), Some(123));
    assert_eq!(parse_percent_to_bp("+12,3%"), Some(1230));
}

#[test]
fn parse_percent_to_bp_bad() {
    assert_eq!(parse_percent_to_bp("1.2.3"), None);
}

#[test]
fn parse_time_duration_ok() {
    assert_eq!(parse_time_duration("10s"), Some(10));
    assert_eq!(parse_time_duration("2m"), Some(120));
    assert_eq!(parse_time_duration("1h"), Some(3600));
    assert_eq!(parse_time_duration("1d"), Some(86_400));
}

#[test]
fn parse_u64_or_none_ok() {
    assert_eq!(parse_u64_or_none("none"), Some(None));
    assert_eq!(parse_u64_or_none("1_000"), Some(Some(1000)));
}

#[test]
fn parse_solana_address_ok() {
    assert!(parse_solana_address("4Nd1mY7NE7CjH2hR5ZQ2Q6F6wW8y1Nf9mKJ5o1Wk").is_some());
}

#[test]
fn format_sol_ok() {
    assert_eq!(format_sol(1_000_000_000), "1 SOL");
    assert_eq!(format_sol(1_200_000_000), "1.2 SOL");
}
