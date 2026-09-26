//! Tracking dashboard state: year, quarter scope, filters, and rows.
//!
//! The backend sends the raw per-year payload (`YearData`); the shared
//! `panya-core` engine computes the verdicts here in wasm. Switching quarters
//! is therefore instant - it recomputes from the same payload and never hits
//! SQL - and the heavy queries run once per fiscal year.

use std::collections::HashMap;

use leptos::prelude::*;
use panya_core::tracking::{Status, Thresholds, TrackingRow, TrackingSummary, track};

use crate::models::YearData;
use crate::services::commands;

/// The status filter applied to the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusFilter {
    /// Every verdict.
    All,
    /// Exactly one verdict.
    Only(Status),
}

impl StatusFilter {
    /// Every filter value, in display order.
    pub const OPTIONS: [Self; 7] = [
        Self::All,
        Self::Only(Status::Behind),
        Self::Only(Status::Watch),
        Self::Only(Status::Unplanned),
        Self::Only(Status::Over),
        Self::Only(Status::NoData),
        Self::Only(Status::OnTrack),
    ];

    /// Thai label for the filter pill.
    #[must_use]
    pub fn label_th(self) -> &'static str {
        match self {
            Self::All => "ทั้งหมด",
            Self::Only(status) => status.label_th(),
        }
    }

    /// Stable key for CSS state classes.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Only(status) => status.key(),
        }
    }

    fn matches(self, status: Status) -> bool {
        match self {
            Self::All => true,
            Self::Only(wanted) => wanted == status,
        }
    }
}

/// The verdicts for the selected scope.
#[derive(Clone, Debug, PartialEq)]
pub struct Computed {
    /// Headline numbers for the KPI strip.
    pub summary: TrackingSummary,
    /// One row per drug (plan lines first, unplanned purchases appended).
    pub rows: Vec<TrackingRow>,
}

/// Shared tracking state.
#[derive(Clone, Copy, Debug)]
pub struct TrackingContext {
    /// Selected fiscal year (CE).
    pub year: RwSignal<i32>,
    /// Selected scope: `0` = ทั้งปี, `1..=4` = quarter.
    pub quarter: RwSignal<u8>,
    /// Selected status filter.
    pub status_filter: RwSignal<StatusFilter>,
    /// Free-text search over code and name.
    pub search: RwSignal<String>,
    /// Fiscal years available in `BUYPLAN_C`.
    pub years: RwSignal<Vec<i32>>,
    /// The raw payload for the selected year (SQL runs once per year).
    pub data: RwSignal<Option<YearData>>,
    /// Verdicts for `data`, scoped to `quarter` - recomputed instantly.
    pub computed: Memo<Option<Computed>>,
    /// `working_code -> position in the computed rows`, built once per
    /// payload/scope change. Stored (not created per call) so every cell
    /// read is O(1).
    index: Memo<HashMap<String, usize>>,
    /// Whether a load is in flight.
    pub loading: RwSignal<bool>,
    /// Last load error.
    pub error: RwSignal<Option<String>>,
    /// `WORKING_CODE` of the row shown in the detail drawer.
    pub selected: RwSignal<Option<String>>,
}

impl TrackingContext {
    /// Create the signals, register them in context, and return the handle.
    #[must_use]
    pub fn provide() -> Self {
        let year = RwSignal::new(0);
        let quarter = RwSignal::new(0);
        let status_filter = RwSignal::new(StatusFilter::All);
        let search = RwSignal::new(String::new());
        let years = RwSignal::new(Vec::new());
        let data: RwSignal<Option<YearData>> = RwSignal::new(None);
        let loading = RwSignal::new(false);
        let error = RwSignal::new(None);
        let selected = RwSignal::new(None);

        // The verdicts for the current scope. Runs the shared engine in
        // wasm; a quarter change recomputes here, no SQL involved.
        let computed = Memo::new(move |_| {
            data.with(|data| {
                data.as_ref().map(|data| {
                    let (rows, summary) = track(
                        &data.plan_lines,
                        &data.actual,
                        quarter.get(),
                        Thresholds::default(),
                    );
                    Computed { summary, rows }
                })
            })
        });

        // Built once here, not per call: a memo created inside `with_row`
        // would rebuild the whole index on every cell read.
        let index = Memo::new(move |_| {
            computed.with(|computed| -> HashMap<String, usize> {
                computed.as_ref().map_or_else(HashMap::new, |computed| {
                    computed
                        .rows
                        .iter()
                        .enumerate()
                        .map(|(index, row)| (row.working_code.clone(), index))
                        .collect()
                })
            })
        });

        let ctx = Self {
            year,
            quarter,
            status_filter,
            search,
            years,
            data,
            computed,
            index,
            loading,
            error,
            selected,
        };
        provide_context(ctx);
        ctx
    }

    /// Fetch the fiscal years available in `BUYPLAN_C`.
    pub async fn fetch_years(self) -> Vec<i32> {
        match commands::invs_get_plan_years().await {
            Ok(years) => {
                self.years.set(years.clone());
                years
            }
            Err(e) => {
                self.error.set(Some(e.message));
                Vec::new()
            }
        }
    }

    /// Load the raw payload for the current year. Quarter changes do not
    /// call this: the shared engine recomputes from the same payload.
    pub async fn load(self) {
        self.loading.set(true);
        self.error.set(None);
        let year = self.year.get_untracked();
        match commands::invs_get_year_data(year).await {
            // Discard a response for a year the operator has already left.
            Ok(data) => {
                if data.year == self.year.get_untracked() {
                    self.data.set(Some(data));
                }
            }
            Err(e) => {
                self.error.set(Some(e.message));
                self.data.set(None);
            }
        }
        self.loading.set(false);
    }

    /// The `WORKING_CODE`s after the status filter and search, sorted by plan.
    ///
    /// The list is what `<For>` diffs: it never clones the rows themselves,
    /// so typing in the search box or reloading the payload only re-diffs
    /// codes, and each row reads its own fields reactively via [`Self::with_row`].
    #[must_use]
    pub fn visible_codes(self) -> Memo<Vec<String>> {
        Memo::new(move |_| {
            let filter = self.status_filter.get();
            let query = self.search.get().trim().to_lowercase();
            self.computed.with(|computed| {
                let Some(computed) = computed.as_ref() else {
                    return Vec::new();
                };
                let mut hits: Vec<(&str, f64)> = computed
                    .rows
                    .iter()
                    .filter(|row| filter.matches(row.status))
                    .filter(|row| {
                        query.is_empty()
                            || row.working_code.to_lowercase().contains(&query)
                            || row.drug_name.to_lowercase().contains(&query)
                    })
                    .map(|row| (row.working_code.as_str(), row.plan_sum))
                    .collect();
                hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                hits.into_iter().map(|(code, _)| code.to_string()).collect()
            })
        })
    }

    /// Run `f` on the computed row for `code` without cloning the whole
    /// payload. Meant to be called inside a reactive closure: it subscribes
    /// to both the computed verdicts and the index, so a reload updates every
    /// cell in place instead of rebuilding the table.
    pub fn with_row<R>(self, code: &str, f: impl FnOnce(Option<&TrackingRow>) -> R) -> R {
        let index = self.index.with(|index| index.get(code).copied());
        self.computed.with(|computed| {
            f(index.and_then(|i| computed.as_ref().and_then(|computed| computed.rows.get(i))))
        })
    }

    /// The row selected in the detail drawer, if it is still present.
    #[must_use]
    pub fn selected_row(self) -> Memo<Option<TrackingRow>> {
        Memo::new(move |_| {
            let code = self.selected.get()?;
            self.with_row(&code, |row| row.cloned())
        })
    }
}
