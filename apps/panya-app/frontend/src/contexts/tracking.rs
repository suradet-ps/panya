//! Tracking dashboard state: year, quarter scope, filters, and rows.

use leptos::prelude::*;
use panya_core::tracking::{Status, TrackingRow};

use crate::models::TrackingResponse;
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
    /// The last loaded payload.
    pub data: RwSignal<Option<TrackingResponse>>,
    /// Monotonic counter bumped on every successful load. The table keys its
    /// rows by `(working_code, revision)`: Leptos `For` reuses a view whose
    /// key is unchanged and never re-renders it with the new item, so without
    /// the revision a year switch would keep the previous year's numbers in
    /// the rows (the codes are mostly the same).
    pub revision: RwSignal<u64>,
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
        let ctx = Self {
            year: RwSignal::new(0),
            quarter: RwSignal::new(0),
            status_filter: RwSignal::new(StatusFilter::All),
            search: RwSignal::new(String::new()),
            years: RwSignal::new(Vec::new()),
            data: RwSignal::new(None),
            revision: RwSignal::new(0),
            loading: RwSignal::new(false),
            error: RwSignal::new(None),
            selected: RwSignal::new(None),
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

    /// Load the tracking payload for the current year and quarter scope.
    pub async fn load(self) {
        self.loading.set(true);
        self.error.set(None);
        let year = self.year.get_untracked();
        let quarter = self.quarter.get_untracked();
        match commands::invs_get_tracking(year, quarter).await {
            // Discard a response that belongs to a scope the operator has
            // already left (fast year/quarter switching); the payload echoes
            // its own year/quarter for exactly this check.
            Ok(data) => {
                if data.year == self.year.get_untracked()
                    && data.quarter == self.quarter.get_untracked()
                {
                    // Bump before storing: the rebuild is triggered by `data`,
                    // and the row key must already read the new revision.
                    self.revision
                        .update(|revision| *revision = revision.wrapping_add(1));
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

    /// The table rows after the status filter and search, sorted by plan.
    #[must_use]
    pub fn rows(self) -> Memo<Vec<TrackingRow>> {
        Memo::new(move |_| {
            let Some(data) = self.data.get() else {
                return Vec::new();
            };
            let filter = self.status_filter.get();
            let query = self.search.get().trim().to_lowercase();
            let mut rows: Vec<TrackingRow> = data
                .rows
                .into_iter()
                .filter(|row| filter.matches(row.status))
                .filter(|row| {
                    query.is_empty()
                        || row.working_code.to_lowercase().contains(&query)
                        || row.drug_name.to_lowercase().contains(&query)
                })
                .collect();
            rows.sort_by(|a, b| {
                b.plan_sum
                    .partial_cmp(&a.plan_sum)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            rows
        })
    }

    /// The row selected in the detail drawer, if it is still present.
    #[must_use]
    pub fn selected_row(self) -> Memo<Option<TrackingRow>> {
        Memo::new(move |_| {
            let code = self.selected.get()?;
            let data = self.data.get()?;
            data.rows.into_iter().find(|row| row.working_code == code)
        })
    }
}
