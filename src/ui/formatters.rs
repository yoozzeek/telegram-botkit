pub const SOL_DISPLAY_DECIMALS: u32 = 5;

pub fn parse_sol_to_lamports(txt: &str) -> Option<u64> {
    let raw = txt.trim();
    if raw.is_empty() {
        return None;
    }

    let mut sep_count = 0usize;

    for ch in raw.chars() {
        match ch {
            '.' | ',' => sep_count += 1,
            '0'..='9' | ' ' | '\u{00A0}' | '_' => {}
            _ => return None,
        }
    }

    if sep_count > 1 {
        return None;
    }

    let dec_pos = raw.find('.').or_else(|| raw.find(','));
    let mut canonical = String::with_capacity(raw.len());

    if let Some(pos) = dec_pos {
        for (i, ch) in raw.chars().enumerate() {
            match ch {
                '0'..='9' => canonical.push(ch),
                '.' | ',' => {
                    if i == pos {
                        canonical.push('.')
                    }
                }
                ' ' | '\u{00A0}' | '_' => {}
                _ => return None,
            }
        }
    } else {
        for ch in raw.chars() {
            match ch {
                '0'..='9' => canonical.push(ch),
                ' ' | '\u{00A0}' | '_' => {}
                _ => return None,
            }
        }
    }

    let mut parts = canonical.split('.');

    let int = parts.next()?;
    if int.is_empty() || !int.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let int_val: u128 = int.parse().ok()?;
    let frac = parts.next();

    if parts.next().is_some() {
        return None;
    }

    let frac_val: u128 = if let Some(fr) = frac {
        if !fr.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }

        let len = fr.len().min(9);
        let mut n: u128 = fr[..len].parse().unwrap_or(0);

        for _ in 0..(9 - len) {
            n *= 10;
        }

        n
    } else {
        0
    };

    let lamports: u128 = int_val.saturating_mul(1_000_000_000) + frac_val;

    u64::try_from(lamports).ok()
}

pub fn parse_percent_to_bp(txt: &str) -> Option<u64> {
    let mut s = String::with_capacity(txt.len());
    let mut saw_sign = false;

    for (i, ch) in txt.chars().enumerate() {
        match ch {
            '%' | ' ' | '\u{00A0}' | '_' => {}
            '+' | '-' if i == 0 && !saw_sign => {
                s.push(ch);
                saw_sign = true;
            }
            _ => s.push(ch),
        }
    }

    let raw = s.trim();
    if raw.is_empty() {
        return None;
    }

    let body = if let Some(first) = raw.chars().next() {
        if first == '-' || first == '+' {
            &raw[1..]
        } else {
            raw
        }
    } else {
        return None;
    };

    if body.is_empty() {
        return None;
    }

    let mut sep_count = 0usize;

    for ch in body.chars() {
        match ch {
            '.' | ',' => sep_count += 1,
            '0'..='9' => {}
            _ => return None,
        }
    }

    if sep_count > 1 {
        return None;
    }

    let dec_pos = body.find('.').or_else(|| body.find(','));
    let mut canonical = String::with_capacity(body.len());

    if let Some(pos) = dec_pos {
        for (i, ch) in body.chars().enumerate() {
            match ch {
                '0'..='9' => canonical.push(ch),
                '.' | ',' => {
                    if i == pos {
                        canonical.push('.')
                    }
                }
                _ => return None,
            }
        }
    } else {
        for ch in body.chars() {
            match ch {
                '0'..='9' => canonical.push(ch),
                _ => return None,
            }
        }
    }

    let mut parts = canonical.split('.');

    let int_s = parts.next()?;
    if int_s.is_empty() || !int_s.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let mut int: u128 = int_s.parse().ok()?;
    let frac = parts.next();

    if parts.next().is_some() {
        return None;
    }

    let mut two: u128 = 0;
    let mut carry = 0u128;

    if let Some(fr) = frac {
        if !fr.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }

        let mut it = fr.chars();

        let d0 = it.next().unwrap_or('0').to_digit(10).unwrap() as u128;
        let d1 = it.next().unwrap_or('0').to_digit(10).unwrap() as u128;
        let d2 = it.next().unwrap_or('0').to_digit(10).unwrap() as u128;

        two = d0 * 10 + d1;

        if d2 >= 5 {
            two += 1;
        }
        if two >= 100 {
            two = 0;
            carry = 1;
        }
    }

    int = int.saturating_add(carry);
    let bp128: u128 = int.saturating_mul(100).saturating_add(two);

    u64::try_from(bp128).ok()
}

pub fn parse_time_duration(txt: &str) -> Option<u64> {
    let s = txt.trim();
    if s.len() < 2 {
        return None;
    }

    let mut buf = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            ' ' | '\u{00A0}' | '_' => {}
            _ => buf.push(ch.to_ascii_lowercase()),
        }
    }

    if buf.len() < 2 {
        return None;
    }

    let (num_str, unit) = match buf.char_indices().next_back() {
        Some((idx, ch)) => (&buf[..idx], ch),
        None => return None,
    };

    if !num_str.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let n: u64 = num_str.parse().ok()?;
    if n == 0 {
        return None;
    }

    match unit {
        's' => Some(n),
        'm' => n.checked_mul(60),
        'h' => n.checked_mul(3600),
        'd' => n.checked_mul(86_400),
        _ => None,
    }
}

pub fn parse_u64_or_none(txt: &str) -> Option<Option<u64>> {
    let t = txt.trim();
    if t.is_empty() {
        return None;
    }

    if t.eq_ignore_ascii_case("none") {
        return Some(None);
    }

    let mut buf = String::with_capacity(t.len());
    for ch in t.chars() {
        match ch {
            '0'..='9' => buf.push(ch),
            ',' | ' ' | '\u{00A0}' | '_' => {}
            _ => return None,
        }
    }

    if buf.is_empty() {
        return None;
    }

    buf.parse::<u64>().ok().map(Some)
}

pub fn parse_solana_address(txt: &str) -> Option<String> {
    let s = txt.trim();
    if s.len() < 32 || s.len() > 44 {
        return None;
    }

    for ch in s.chars() {
        match ch {
            '1'..='9' | 'A'..='H' | 'J'..='N' | 'P'..='Z' | 'a'..='k' | 'm'..='z' => {}
            _ => return None,
        }
    }

    Some(s.to_string())
}

pub fn format_duration_short(secs: u64) -> String {
    const DAY: u64 = 86_400;
    const HOUR: u64 = 3_600;
    const MIN: u64 = 60;

    if secs % DAY == 0 {
        return format!("{}d", secs / DAY);
    }
    if secs % HOUR == 0 {
        return format!("{}h", secs / HOUR);
    }
    if secs % MIN == 0 {
        return format!("{}m", secs / MIN);
    }

    format!("{secs}s")
}

pub fn format_sol(lamports: u64) -> String {
    const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

    let whole = lamports / LAMPORTS_PER_SOL;
    let frac_lamports = lamports % LAMPORTS_PER_SOL;

    if frac_lamports == 0 {
        return format!("{whole} SOL");
    }

    let mut frac_str = format!("{frac_lamports:09}");
    while frac_str.ends_with('0') {
        frac_str.pop();
    }

    if frac_str.is_empty() {
        return format!("{whole} SOL");
    }

    format!("{whole}.{frac_str} SOL")
}
