use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::config::WeekStart;
use crate::model::Todo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreakRule {
    #[default]
    All,
    Any,
}

impl StreakRule {
    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "all" => Some(StreakRule::All),
            "any" => Some(StreakRule::Any),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaySummary {
    pub date: NaiveDate,
    pub done: usize,
    pub total: usize,
}

impl DaySummary {
    pub fn is_complete(&self, rule: StreakRule) -> bool {
        if self.total == 0 {
            return false;
        }
        match rule {
            StreakRule::All => self.done == self.total,
            StreakRule::Any => self.done > 0,
        }
    }

    pub fn percent(&self) -> Option<usize> {
        (self.done * 100).checked_div(self.total)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PeriodSummary {
    pub done: usize,
    pub total: usize,
    pub percent: usize,
}

/// Computes DaySummary for each date in `dates`.
/// Counting rule: top-level tasks only (`t.parent.is_none()`).
pub fn day_summaries(
    todos: &[Todo],
    dates: impl IntoIterator<Item = NaiveDate>,
) -> Vec<DaySummary> {
    let mut map: std::collections::HashMap<NaiveDate, (usize, usize)> =
        std::collections::HashMap::new();
    for t in todos {
        if t.parent.is_none() {
            let entry = map.entry(t.created_date).or_insert((0, 0));
            entry.1 += 1;
            if t.done {
                entry.0 += 1;
            }
        }
    }

    dates
        .into_iter()
        .map(|d| {
            let (done, total) = map.get(&d).copied().unwrap_or((0, 0));
            DaySummary {
                date: d,
                done,
                total,
            }
        })
        .collect()
}

/// Weighted period summary across multiple DaySummary items.
/// Integer percentage rounded down; 0 when total tasks is 0.
pub fn period_summary(summaries: &[DaySummary]) -> PeriodSummary {
    let mut total_done = 0;
    let mut total_tasks = 0;
    for s in summaries {
        total_done += s.done;
        total_tasks += s.total;
    }
    let percent = (total_done * 100).checked_div(total_tasks).unwrap_or(0);
    PeriodSummary {
        done: total_done,
        total: total_tasks,
        percent,
    }
}

/// Calculates current streak:
/// Consecutive complete days ending today if today is complete, otherwise ending yesterday.
/// An unfinished today does NOT break the streak, but a past day that is not complete
/// (including a day with no tasks) does. Future days are ignored.
pub fn current_streak(todos: &[Todo], today: NaiveDate, rule: StreakRule) -> usize {
    let mut map: std::collections::HashMap<NaiveDate, (usize, usize)> =
        std::collections::HashMap::new();
    for t in todos {
        if t.parent.is_none() && t.created_date <= today {
            let entry = map.entry(t.created_date).or_insert((0, 0));
            entry.1 += 1;
            if t.done {
                entry.0 += 1;
            }
        }
    }

    let is_day_complete = |d: NaiveDate| -> bool {
        if let Some(&(done, total)) = map.get(&d) {
            if total == 0 {
                false
            } else {
                match rule {
                    StreakRule::All => done == total,
                    StreakRule::Any => done > 0,
                }
            }
        } else {
            false
        }
    };

    if is_day_complete(today) {
        let mut streak = 1;
        let mut cur = today.pred_opt();
        while let Some(d) = cur {
            if is_day_complete(d) {
                streak += 1;
                cur = d.pred_opt();
            } else {
                break;
            }
        }
        streak
    } else {
        let mut streak = 0;
        let mut cur = today.pred_opt();
        while let Some(d) = cur {
            if is_day_complete(d) {
                streak += 1;
                cur = d.pred_opt();
            } else {
                break;
            }
        }
        streak
    }
}

/// Calculates longest streak in all history (dates <= today).
pub fn longest_streak(todos: &[Todo], today: NaiveDate, rule: StreakRule) -> usize {
    let mut map: std::collections::HashMap<NaiveDate, (usize, usize)> =
        std::collections::HashMap::new();
    for t in todos {
        if t.parent.is_none() && t.created_date <= today {
            let entry = map.entry(t.created_date).or_insert((0, 0));
            entry.1 += 1;
            if t.done {
                entry.0 += 1;
            }
        }
    }

    let mut complete_dates: Vec<NaiveDate> = map
        .into_iter()
        .filter(|&(_d, (done, total))| {
            if total == 0 {
                false
            } else {
                match rule {
                    StreakRule::All => done == total,
                    StreakRule::Any => done > 0,
                }
            }
        })
        .map(|(d, _)| d)
        .collect();

    if complete_dates.is_empty() {
        return 0;
    }

    complete_dates.sort_unstable();

    let mut max_streak = 0;
    let mut current_run = 0;
    let mut prev_date: Option<NaiveDate> = None;

    for date in complete_dates {
        if let Some(prev) = prev_date {
            if prev.succ_opt() == Some(date) {
                current_run += 1;
            } else {
                current_run = 1;
            }
        } else {
            current_run = 1;
        }
        if current_run > max_streak {
            max_streak = current_run;
        }
        prev_date = Some(date);
    }

    max_streak
}

#[derive(Debug, Clone, Default)]
pub struct CachedStats {
    pub current_streak: usize,
    pub longest_streak: usize,
    pub this_week_summary: PeriodSummary,
    pub last_30_days_summary: PeriodSummary,
}

#[derive(Debug, Clone, Default)]
pub struct StatsCache {
    revision: Option<u64>,
    cached_today: Option<NaiveDate>,
    cached_rule: Option<StreakRule>,
    cached_week_start: Option<WeekStart>,
    cached_stats: CachedStats,
}

impl StatsCache {
    pub fn invalidate(&mut self) {
        self.revision = None;
    }

    pub fn get_or_update(
        &mut self,
        current_revision: u64,
        today: NaiveDate,
        todos: &[Todo],
        rule: StreakRule,
        week_start: WeekStart,
    ) -> &CachedStats {
        if self.revision == Some(current_revision)
            && self.cached_today == Some(today)
            && self.cached_rule == Some(rule)
            && self.cached_week_start == Some(week_start)
        {
            return &self.cached_stats;
        }

        let cur_streak = current_streak(todos, today, rule);
        let long_streak = longest_streak(todos, today, rule);

        // this week
        let w_start = crate::dates::week_start_of(today, week_start);
        let w_days = crate::dates::week_days(w_start);
        let this_week_summaries = day_summaries(todos, w_days);
        let this_week_summary = period_summary(&this_week_summaries);

        // last 30 days: (today - 29 days) ..= today
        let start_30 = today - chrono::Duration::days(29);
        let mut days_30 = Vec::with_capacity(30);
        let mut cur = start_30;
        while cur <= today {
            days_30.push(cur);
            if let Some(next) = cur.succ_opt() {
                cur = next;
            } else {
                break;
            }
        }
        let last_30_summaries = day_summaries(todos, days_30);
        let last_30_days_summary = period_summary(&last_30_summaries);

        self.cached_stats = CachedStats {
            current_streak: cur_streak,
            longest_streak: long_streak,
            this_week_summary,
            last_30_days_summary,
        };
        self.revision = Some(current_revision);
        self.cached_today = Some(today);
        self.cached_rule = Some(rule);
        self.cached_week_start = Some(week_start);

        &self.cached_stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_summaries_subtasks_ignored() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let todos = vec![
            Todo {
                id: 1,
                text: "Top 1".to_string(),
                created_date: date,
                done: true,
                parent: None,
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Subtask 1".to_string(),
                created_date: date,
                done: false,
                parent: Some(1),
                ..Default::default()
            },
            Todo {
                id: 3,
                text: "Top 2".to_string(),
                created_date: date,
                done: false,
                parent: None,
                ..Default::default()
            },
        ];

        let summaries = day_summaries(&todos, vec![date]);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].total, 2);
        assert_eq!(summaries[0].done, 1);
        assert_eq!(summaries[0].percent(), Some(50));
    }

    #[test]
    fn test_period_summary_weighted_and_zero_guard() {
        let empty_summary = period_summary(&[]);
        assert_eq!(empty_summary.total, 0);
        assert_eq!(empty_summary.done, 0);
        assert_eq!(empty_summary.percent, 0);

        let d1 = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let summaries = vec![
            DaySummary {
                date: d1,
                done: 1,
                total: 1,
            }, // 100%
            DaySummary {
                date: d2,
                done: 0,
                total: 3,
            }, // 0%
        ];
        // Average of percentages would be 50%, but weighted is 1/4 = 25%
        let p = period_summary(&summaries);
        assert_eq!(p.done, 1);
        assert_eq!(p.total, 4);
        assert_eq!(p.percent, 25);
    }

    #[test]
    fn test_streaks_all_vs_any_and_edge_cases() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap(); // Friday
        let d_thu = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let d_wed = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
        let d_tue = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();

        // 1. Empty data: streaks are 0
        assert_eq!(current_streak(&[], today, StreakRule::All), 0);
        assert_eq!(longest_streak(&[], today, StreakRule::All), 0);

        // 2. Wed: 2/2 done. Thu: 1/2 done. Today (Fri): 0 tasks.
        let todos = vec![
            Todo {
                id: 1,
                created_date: d_wed,
                done: true,
                parent: None,
                ..Default::default()
            },
            Todo {
                id: 2,
                created_date: d_wed,
                done: true,
                parent: None,
                ..Default::default()
            },
            Todo {
                id: 3,
                created_date: d_thu,
                done: true,
                parent: None,
                ..Default::default()
            },
            Todo {
                id: 4,
                created_date: d_thu,
                done: false,
                parent: None,
                ..Default::default()
            },
        ];

        // Rule All: Thu is not complete (1/2), so streak ending Thu is 0
        assert_eq!(current_streak(&todos, today, StreakRule::All), 0);
        // But longest streak in history was Wed (1 day)
        assert_eq!(longest_streak(&todos, today, StreakRule::All), 1);

        // Rule Any: Thu is complete (done > 0), Wed is complete (done > 0). Today is unfinished (0 tasks).
        // Streak ending yesterday is 2!
        assert_eq!(current_streak(&todos, today, StreakRule::Any), 2);
        assert_eq!(longest_streak(&todos, today, StreakRule::Any), 2);

        // 3. Gap day breaks streak:
        // Tue: complete (1/1). Wed: NO tasks. Thu: complete (1/1). Today: incomplete (0/1).
        let todos_gap = vec![
            Todo {
                id: 1,
                created_date: d_tue,
                done: true,
                parent: None,
                ..Default::default()
            },
            Todo {
                id: 2,
                created_date: d_thu,
                done: true,
                parent: None,
                ..Default::default()
            },
            Todo {
                id: 3,
                created_date: today,
                done: false,
                parent: None,
                ..Default::default()
            },
        ];
        // Today is incomplete -> streak ends yesterday (Thu).
        // Thu was complete (1). Wed had no tasks -> breaks streak!
        assert_eq!(current_streak(&todos_gap, today, StreakRule::All), 1);
        assert_eq!(longest_streak(&todos_gap, today, StreakRule::All), 1);

        // 4. Today complete counts:
        let mut todos_today_complete = todos_gap.clone();
        todos_today_complete[2].done = true;
        // Today is complete (1) + Thu was complete (1) + Wed had no tasks (0) => streak is 2 (Today + Thu)
        assert_eq!(
            current_streak(&todos_today_complete, today, StreakRule::All),
            2
        );
        assert_eq!(
            longest_streak(&todos_today_complete, today, StreakRule::All),
            2
        );

        // 5. Future tasks are ignored for streaks
        let d_future = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        let mut todos_future = todos_today_complete;
        todos_future.push(Todo {
            id: 99,
            created_date: d_future,
            done: true,
            parent: None,
            ..Default::default()
        });
        assert_eq!(current_streak(&todos_future, today, StreakRule::All), 2);
        assert_eq!(longest_streak(&todos_future, today, StreakRule::All), 2);
    }

    #[test]
    fn test_stats_cache_invalidation() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut cache = StatsCache::default();
        let todos = vec![Todo {
            id: 1,
            created_date: today,
            done: true,
            parent: None,
            ..Default::default()
        }];

        let s1 = cache
            .get_or_update(1, today, &todos, StreakRule::All, WeekStart::Monday)
            .clone();
        assert_eq!(s1.current_streak, 1);

        // Same revision -> returns cached
        let s2 = cache
            .get_or_update(1, today, &[], StreakRule::All, WeekStart::Monday)
            .clone();
        assert_eq!(s2.current_streak, 1);

        // Different revision -> recomputes
        let s3 = cache
            .get_or_update(2, today, &[], StreakRule::All, WeekStart::Monday)
            .clone();
        assert_eq!(s3.current_streak, 0);

        // Invalidate -> recomputes even with same revision
        cache.invalidate();
        let s4 = cache
            .get_or_update(2, today, &todos, StreakRule::All, WeekStart::Monday)
            .clone();
        assert_eq!(s4.current_streak, 1);
    }
}
