//! Thai fiscal-year and quarter arithmetic.
//!
//! The Thai fiscal year runs 1 Oct → 30 Sep (FY N starts 1 Oct of N−1) and
//! its four quarters are calendar-aligned:
//!
//! | Quarter | Months | Fiscal months |
//! |---------|--------|---------------|
//! | Q1 | ต.ค.–ธ.ค. | 1–3 |
//! | Q2 | ม.ค.–มี.ค. | 4–6 |
//! | Q3 | เม.ย.–มิ.ย. | 7–9 |
//! | Q4 | ก.ค.–ก.ย. | 10–12 |
//!
//! Every calendar ↔ fiscal mapping lives here so the SQL layer, the tracking
//! engine, and the UI agree on one axis. Pure functions, unit-tested.

/// Number of quarters in a fiscal year (array width used by the engine).
pub const QUARTERS: usize = 4;

/// The fiscal-year window as `YYYYMMDD` integers (INVS `RECEIVE_DATE` shape):
/// FY `fy` = 1 Oct (`fy`−1) … 30 Sep `fy`.
///
/// This is the single definition of the boundary: a record on 30 Sep `fy`
/// belongs to FY `fy`; a record on 1 Oct `fy` belongs to FY `fy`+1.
#[must_use]
pub fn fiscal_year_range(fy: i32) -> (i32, i32) {
    ((fy - 1) * 10_000 + 1001, fy * 10_000 + 930)
}

/// The `YYYYMMDD` window of one quarter (`1..=4`) of fiscal year `fy`.
///
/// An out-of-range quarter degrades to the whole fiscal year, so a caller
/// that passes `0` ("ทั้งปี") gets the full window.
#[must_use]
pub fn quarter_range(fy: i32, quarter: u8) -> (i32, i32) {
    match quarter {
        1 => ((fy - 1) * 10_000 + 1001, (fy - 1) * 10_000 + 1231),
        2 => (fy * 10_000 + 101, fy * 10_000 + 331),
        3 => (fy * 10_000 + 401, fy * 10_000 + 630),
        4 => (fy * 10_000 + 701, fy * 10_000 + 930),
        _ => fiscal_year_range(fy),
    }
}

/// Map a calendar month (1–12) to its 1-based fiscal quarter, or `None` for
/// an out-of-range month.
#[must_use]
pub fn cal_month_to_quarter(cal_month: u32) -> Option<u8> {
    match cal_month {
        10..=12 => Some(1),
        1..=3 => Some(2),
        4..=6 => Some(3),
        7..=9 => Some(4),
        _ => None,
    }
}

/// Map a calendar month to the 0-based quarter index used by [`QUARTERS`]
/// arrays, or `None` for an out-of-range month.
#[must_use]
pub fn cal_month_to_quarter_idx(cal_month: u32) -> Option<usize> {
    cal_month_to_quarter(cal_month).map(|q| usize::from(q - 1))
}

/// The three calendar months of `quarter` (`1..=4`), or `None` when the
/// quarter is out of range.
#[must_use]
pub fn quarter_months(quarter: u8) -> Option<[u32; 3]> {
    match quarter {
        1 => Some([10, 11, 12]),
        2 => Some([1, 2, 3]),
        3 => Some([4, 5, 6]),
        4 => Some([7, 8, 9]),
        _ => None,
    }
}

/// Thai label for a quarter: `ต.ค.–ธ.ค.` … `ก.ค.–ก.ย.`, or `""` when the
/// quarter is out of range. `0` means "ทั้งปี" and renders as such.
#[must_use]
pub fn quarter_label(quarter: u8) -> &'static str {
    match quarter {
        0 => "ทั้งปี",
        1 => "ต.ค.–ธ.ค.",
        2 => "ม.ค.–มี.ค.",
        3 => "เม.ย.–มิ.ย.",
        4 => "ก.ค.–ก.ย.",
        _ => "",
    }
}

/// Thai short labels for the 12 fiscal months (index 0 = ต.ค.).
pub const FISCAL_MONTHS_SHORT: [&str; 12] = [
    "ต.ค.",
    "พ.ย.",
    "ธ.ค.",
    "ม.ค.",
    "ก.พ.",
    "มี.ค.",
    "เม.ย.",
    "พ.ค.",
    "มิ.ย.",
    "ก.ค.",
    "ส.ค.",
    "ก.ย.",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fiscal_year_range_covers_oct_to_sep() {
        // FY 2027 = 1 Oct 2026 … 30 Sep 2027.
        assert_eq!(fiscal_year_range(2027), (20261001, 20270930));
    }

    #[test]
    fn fiscal_boundary_months_belong_to_the_right_year() {
        let (start, end) = fiscal_year_range(2027);
        assert!(20260930 < start, "30 Sep 2026 is FY2026");
        assert!((20261001..=20270930).contains(&start));
        assert_eq!(end, 20270930, "30 Sep 2027 closes FY2027");
    }

    #[test]
    fn quarter_ranges_tile_the_fiscal_year() {
        assert_eq!(quarter_range(2027, 1), (20261001, 20261231));
        assert_eq!(quarter_range(2027, 2), (20270101, 20270331));
        assert_eq!(quarter_range(2027, 3), (20270401, 20270630));
        assert_eq!(quarter_range(2027, 4), (20270701, 20270930));
    }

    #[test]
    fn quarter_range_zero_degrades_to_the_whole_year() {
        assert_eq!(quarter_range(2027, 0), fiscal_year_range(2027));
        assert_eq!(quarter_range(2027, 9), fiscal_year_range(2027));
    }

    #[test]
    fn calendar_month_maps_to_the_confirmed_quarters() {
        // Q1 = Oct–Dec, Q2 = Jan–Mar, Q3 = Apr–Jun, Q4 = Jul–Sep.
        for month in [10, 11, 12] {
            assert_eq!(cal_month_to_quarter(month), Some(1));
        }
        for month in [1, 2, 3] {
            assert_eq!(cal_month_to_quarter(month), Some(2));
        }
        for month in [4, 5, 6] {
            assert_eq!(cal_month_to_quarter(month), Some(3));
        }
        for month in [7, 8, 9] {
            assert_eq!(cal_month_to_quarter(month), Some(4));
        }
        assert_eq!(cal_month_to_quarter(0), None);
        assert_eq!(cal_month_to_quarter(13), None);
    }

    #[test]
    fn quarter_months_concatenate_into_a_fiscal_year() {
        let mut fiscal_months: Vec<u32> = Vec::new();
        for q in 1..=4u8 {
            fiscal_months.extend(quarter_months(q).expect("valid quarter"));
        }
        assert_eq!(fiscal_months, vec![10, 11, 12, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(quarter_months(5), None);
    }

    #[test]
    fn quarter_labels_are_thai() {
        assert_eq!(quarter_label(0), "ทั้งปี");
        assert_eq!(quarter_label(1), "ต.ค.–ธ.ค.");
        assert_eq!(quarter_label(4), "ก.ค.–ก.ย.");
        assert_eq!(quarter_label(9), "");
    }
}
