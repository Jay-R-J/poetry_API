//! 每日一诗：按日期确定性选取，同一天返回同一首。

use chrono::Local;

/// 返回今天的日期字符串（`YYYY-MM-DD`）。
pub fn date_string() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// 由日期字符串得到稳定索引（`0..total`），用于按偏移量取诗。
///
/// 使用 FNV-1a 哈希，跨进程/跨平台结果一致（不能用 `std::collections::HashMap`
/// 的默认哈希器，其种子是随机的）。
pub fn stable_index(date: &str, total: usize) -> usize {
    if total == 0 {
        return 0;
    }
    (fnv1a(date.as_bytes()) % total as u64) as usize
}

/// FNV-1a 64 位哈希。
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_index_is_deterministic() {
        assert_eq!(
            stable_index("2026-09-05", 100),
            stable_index("2026-09-05", 100)
        );
    }

    #[test]
    fn stable_index_stays_within_bounds() {
        for date in ["2026-01-01", "2026-06-15", "2026-12-31"] {
            let idx = stable_index(date, 100);
            assert!(idx < 100, "index {idx} out of bounds");
        }
    }

    #[test]
    fn stable_index_zero_total_is_safe() {
        assert_eq!(stable_index("2026-09-05", 0), 0);
    }
}
