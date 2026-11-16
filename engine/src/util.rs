//! 端数処理・表示用の小物

/// 1万円
pub const MAN: i64 = 10_000;

/// `unit` 未満を切り捨て（マイナスは 0 扱い）
pub fn floor_to(value: i64, unit: i64) -> i64 {
    if value <= 0 || unit <= 0 {
        0
    } else {
        value / unit * unit
    }
}

/// 切り上げ割り算（b <= 0 のときは 0）
pub fn ceil_div(a: i64, b: i64) -> i64 {
    if b <= 0 || a <= 0 {
        0
    } else {
        (a + b - 1) / b
    }
}

/// 3桁区切り
pub fn with_commas(v: i64) -> String {
    let neg = v < 0;
    let s = v.unsigned_abs().to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if neg {
        format!("-{out}")
    } else {
        out
    }
}

/// メッセージ用の金額表示。1万円以上は「12.3万円」、未満は「3,000円」
pub fn fmt_yen(v: i64) -> String {
    let v = v.max(0);
    if v >= MAN {
        let tenths = (v + 500) / 1_000; // 0.1万円単位で四捨五入
        let whole = tenths / 10;
        let frac = tenths % 10;
        if frac == 0 {
            format!("{}万円", with_commas(whole))
        } else {
            format!("{}.{}万円", with_commas(whole), frac)
        }
    } else {
        format!("{}円", with_commas(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(fmt_yen(3_000), "3,000円");
        assert_eq!(fmt_yen(10_000), "1万円");
        assert_eq!(fmt_yen(109_000), "10.9万円");
        assert_eq!(fmt_yen(12_345_678), "1,234.6万円");
        assert_eq!(with_commas(1_234_567), "1,234,567");
        assert_eq!(floor_to(12_345, 100), 12_300);
        assert_eq!(floor_to(-5, 100), 0);
        assert_eq!(ceil_div(10, 3), 4);
    }
}
