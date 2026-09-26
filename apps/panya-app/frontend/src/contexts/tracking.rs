//! Tracking dashboard state: year, quarter scope, filters, sorting, and rows.
//!
//! The backend sends the raw per-year payload (`YearData`); the shared
//! `panya-core` engine computes the verdicts here in wasm. Switching quarters
//! is therefore instant - it recomputes from the same payload and never hits
//! SQL - and the heavy queries run once per fiscal year.

use std::collections::HashMap;
use std::sync::{Arc, PoisonError, RwLock};

use leptos::prelude::*;
use panya_core::tracking::{Status, Thresholds, TrackingRow, TrackingSummary, track};

use crate::models::YearData;
use crate::services::commands;
use crate::services::timers::set_timeout_ms;

/// How long the search box waits after the last keystroke before filtering.
const SEARCH_DEBOUNCE_MS: i32 = 250;

/// Loading indicators only appear after this delay. A fast load (under the
/// threshold) must show nothing at all - a skeleton that flashes for 100 ms
/// reads as jank, not as feedback.
const LOADING_INDICATOR_DELAY_MS: i32 = 300;

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

/// The column the table is sorted by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    /// Drug name.
    Name,
    /// Plan value within the scope.
    Plan,
    /// Actual value within the scope.
    Actual,
    /// Achievement percentage.
    Pct,
    /// Verdict (most severe first when ascending).
    Status,
}

/// Severity order used when sorting by verdict.
fn status_rank(status: Status) -> u8 {
    match status {
        Status::Behind => 0,
        Status::Watch => 1,
        Status::Unplanned => 2,
        Status::Over => 3,
        Status::NoData => 4,
        Status::OnTrack => 5,
    }
}

/// Total-order comparison for values that are never NaN in practice.
fn cmp_f64(a: f64, b: f64) -> std::cmp::Ordering {
    a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
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
    /// The search box text as typed (debounced into [`Self::search`]).
    pub search_input: RwSignal<String>,
    /// The applied search text.
    pub search: RwSignal<String>,
    /// Sort column and direction.
    pub sort: RwSignal<SortKey>,
    /// `true` = ascending.
    pub sort_asc: RwSignal<bool>,
    /// Fiscal years available in `BUYPLAN_C`.
    pub years: RwSignal<Vec<i32>>,
    /// The raw payload for the selected year (SQL runs once per year).
    pub data: RwSignal<Option<YearData>>,
    /// Verdicts for `data`, scoped to `quarter`, behind an `Arc` so the
    /// per-quarter cache below hands out cheap clones.
    pub computed: Memo<Option<Arc<Computed>>>,
    /// `working_code -> position in the computed rows`, built once per
    /// payload/scope change. Stored (not created per call) so every cell
    /// read is O(1).
    index: Memo<HashMap<String, usize>>,
    /// The `WORKING_CODE`s after the status filter and search, in sort
    /// order - what the table's `<For>` diffs and the result count shows.
    pub visible: Memo<Vec<String>>,
    /// Whether a load is in flight.
    pub loading: RwSignal<bool>,
    /// `loading`, delayed by [`LOADING_INDICATOR_DELAY_MS`] - what the UI
    /// actually binds to, so fast loads never flash an indicator.
    pub loading_visible: RwSignal<bool>,
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
        let search_input = RwSignal::new(String::new());
        let search = RwSignal::new(String::new());
        let sort = RwSignal::new(SortKey::Plan);
        let sort_asc = RwSignal::new(false);
        let years = RwSignal::new(Vec::new());
        let data: RwSignal<Option<YearData>> = RwSignal::new(None);
        let loading = RwSignal::new(false);
        let loading_visible = RwSignal::new(false);
        let error = RwSignal::new(None);
        let selected = RwSignal::new(None);

        // Delay the loading indicator: if the load finishes before the
        // threshold, nothing ever appears. The generation guard cancels a
        // pending timer when the load ends (or a new one starts).
        let load_generation = RwSignal::new(0_u64);
        Effect::new(move |_| {
            if loading.get() {
                let generation = load_generation.get_untracked().wrapping_add(1);
                load_generation.set(generation);
                set_timeout_ms(
                    move || {
                        if load_generation.get_untracked() == generation && loading.get_untracked()
                        {
                            loading_visible.set(true);
                        }
                    },
                    LOADING_INDICATOR_DELAY_MS,
                );
            } else {
                load_generation.update(|generation| *generation = generation.wrapping_add(1));
                loading_visible.set(false);
            }
        });

        // Debounce the search box: filter only after the typing pauses, and
        // drop stale timers with a generation guard.
        let debounce_generation = RwSignal::new(0_u64);
        Effect::new(move |_| {
            let text = search_input.get();
            let generation = debounce_generation.get_untracked().wrapping_add(1);
            debounce_generation.set(generation);
            set_timeout_ms(
                move || {
                    if debounce_generation.get_untracked() == generation {
                        search.set(text.clone());
                    }
                },
                SEARCH_DEBOUNCE_MS,
            );
        });

        // The verdicts for the current scope, cached per quarter until the
        // payload changes: switching back and forth between quarters must
        // never re-run the engine (that reallocation is what made a click
        // feel sticky).
        let quarter_cache: StoredValue<RwLock<HashMap<u8, Arc<Computed>>>> =
            StoredValue::new(RwLock::new(HashMap::new()));
        Effect::new(move |_| {
            let _ = data.get();
            quarter_cache.with_value(|cache| {
                cache
                    .write()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clear();
            });
        });

        let computed = Memo::new(move |_| {
            data.with(|data| {
                let data = data.as_ref()?;
                let quarter = quarter.get();
                if let Some(cached) = quarter_cache.with_value(|cache| {
                    cache
                        .read()
                        .unwrap_or_else(PoisonError::into_inner)
                        .get(&quarter)
                        .cloned()
                }) {
                    return Some(cached);
                }
                let (rows, summary) = track(
                    &data.plan_lines,
                    &data.actual,
                    quarter,
                    Thresholds::default(),
                );
                let computed = Arc::new(Computed { summary, rows });
                quarter_cache.with_value(|cache| {
                    cache
                        .write()
                        .unwrap_or_else(PoisonError::into_inner)
                        .insert(quarter, Arc::clone(&computed));
                });
                Some(computed)
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

        // The visible list (filter + search + sort) is what the table's
        // `<For>` diffs: it never clones the rows themselves, so typing in
        // the search box or reloading the payload only re-diffs codes, and
        // each row reads its own fields reactively via `with_row`.
        let visible = Memo::new(move |_| {
            let filter = status_filter.get();
            let query = search.get().trim().to_lowercase();
            let sort = sort.get();
            let ascending = sort_asc.get();
            computed.with(|computed| {
                let Some(computed) = computed.as_ref() else {
                    return Vec::new();
                };
                let mut hits: Vec<&TrackingRow> = computed
                    .rows
                    .iter()
                    .filter(|row| filter.matches(row.status))
                    .filter(|row| {
                        query.is_empty()
                            || row.working_code.to_lowercase().contains(&query)
                            || row.drug_name.to_lowercase().contains(&query)
                    })
                    .collect();
                hits.sort_by(|a, b| {
                    let ordering = match sort {
                        SortKey::Name => a.drug_name.cmp(&b.drug_name),
                        SortKey::Plan => cmp_f64(a.plan_sum, b.plan_sum),
                        SortKey::Actual => cmp_f64(a.actual_sum, b.actual_sum),
                        SortKey::Pct => cmp_f64(
                            a.achievement_pct.unwrap_or(f64::NEG_INFINITY),
                            b.achievement_pct.unwrap_or(f64::NEG_INFINITY),
                        ),
                        SortKey::Status => status_rank(a.status).cmp(&status_rank(b.status)),
                    };
                    let ordering = if ascending {
                        ordering
                    } else {
                        ordering.reverse()
                    };
                    ordering.then_with(|| a.working_code.cmp(&b.working_code))
                });
                hits.into_iter()
                    .map(|row| row.working_code.clone())
                    .collect()
            })
        });

        let ctx = Self {
            year,
            quarter,
            status_filter,
            search_input,
            search,
            sort,
            sort_asc,
            years,
            data,
            computed,
            index,
            visible,
            loading,
            loading_visible,
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

    /// Clear the status filter and the search box (the empty-state action).
    pub fn clear_filters(self) {
        self.status_filter.set(StatusFilter::All);
        self.search_input.set(String::new());
        self.search.set(String::new());
    }

    /// Whether a filter or search is currently narrowing the table.
    #[must_use]
    pub fn filters_active(self) -> Memo<bool> {
        Memo::new(move |_| {
            self.status_filter.get() != StatusFilter::All
                || !self.search_input.get().trim().is_empty()
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
