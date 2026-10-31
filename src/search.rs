use chrono::NaiveDate;

use crate::model::Todo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub task_id: u64,
    pub date: NaiveDate,
    pub is_subtask: bool,
    pub parent_id: Option<u64>,
    pub display_text: String,
    pub done: bool,
    pub priority: u8,
    pub tags: Vec<String>,
    pub has_note: bool,
    pub match_in_note_only: bool,
    pub is_word_prefix_match: bool,
    pub highlight_ranges: Vec<(usize, usize)>, // [start_char_idx, end_char_idx) in display_text
    pub original_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    pub total_hits: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryToken {
    PriorityMin(u8),
    DoneStatus(bool),
    Tag(String),
    Plain(String),
}

/// Parses search query string into structured query tokens.
pub fn parse_query(query: &str) -> Vec<QueryToken> {
    let mut tokens = Vec::new();
    for word in query.split_whitespace() {
        if word == "!" {
            tokens.push(QueryToken::PriorityMin(1));
        } else if word == "!!" {
            tokens.push(QueryToken::PriorityMin(2));
        } else if word.eq_ignore_ascii_case("is:done") {
            tokens.push(QueryToken::DoneStatus(true));
        } else if word.eq_ignore_ascii_case("is:open") {
            tokens.push(QueryToken::DoneStatus(false));
        } else if let Some(tag) = word.strip_prefix('#') {
            if !tag.is_empty() {
                tokens.push(QueryToken::Tag(tag.to_lowercase()));
            }
        } else {
            tokens.push(QueryToken::Plain(word.to_string()));
        }
    }
    tokens
}

/// Unicode-safe substring matcher returning character index ranges `(start, end)` in `haystack`.
/// Does not slice byte offsets from lowercased copies, ensuring complete Unicode safety.
pub fn find_char_ranges(haystack: &str, needle: &str) -> Vec<(usize, usize)> {
    if needle.is_empty() {
        return Vec::new();
    }
    let haystack_chars: Vec<char> = haystack.chars().collect();
    let needle_lower = needle.to_lowercase();
    let needle_char_count = needle_lower.chars().count();
    let mut ranges = Vec::new();

    let h_len = haystack_chars.len();
    let min_k = needle_char_count.saturating_sub(2).max(1);
    let max_k = needle_char_count + 4;

    for i in 0..h_len {
        for k in min_k..=max_k {
            let j = i + k;
            if j > h_len {
                break;
            }
            let sub: String = haystack_chars[i..j].iter().collect();
            if sub.to_lowercase() == needle_lower {
                ranges.push((i, j));
                break;
            }
        }
    }

    ranges
}

/// Merges overlapping and adjacent character ranges.
pub fn merge_ranges(mut ranges: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    if ranges.is_empty() {
        return ranges;
    }
    ranges.sort_unstable_by_key(|r| (r.0, r.1));
    let mut merged = Vec::with_capacity(ranges.len());
    let mut cur = ranges[0];

    for r in ranges.into_iter().skip(1) {
        if r.0 <= cur.1 {
            cur.1 = cur.1.max(r.1);
        } else {
            merged.push(cur);
            cur = r;
        }
    }
    merged.push(cur);
    merged
}

/// Returns true if a match starting at char index `start` in `chars` is at a word boundary.
fn is_boundary_match(chars: &[char], start: usize) -> bool {
    if start == 0 {
        return true;
    }
    let prev = chars[start - 1];
    prev.is_whitespace()
        || prev == '›'
        || prev == '>'
        || prev == '-'
        || prev == '_'
        || prev == '/'
        || prev == ':'
        || prev == '.'
        || prev == ','
}

/// Computes task count and distinct day count for the empty search hint.
pub fn search_stats(todos: &[Todo], today: NaiveDate, show_history: bool) -> (usize, usize) {
    let mut dates = std::collections::BTreeSet::new();
    let mut task_count = 0;
    for t in todos {
        if show_history || t.created_date >= today {
            task_count += 1;
            dates.insert(t.created_date);
        }
    }
    (task_count, dates.len())
}

/// Searches `todos` across all days according to query and configuration.
/// Deterministically ranked: word-prefix matches first, then newest date first, then manual order.
/// Result hits are capped at 200.
pub fn search_tasks(
    todos: &[Todo],
    query: &str,
    today: NaiveDate,
    show_history: bool,
    is_ascii: bool,
) -> SearchResults {
    let tokens = parse_query(query);
    if tokens.is_empty() {
        return SearchResults {
            hits: Vec::new(),
            total_hits: 0,
        };
    }

    let separator = if is_ascii { ">" } else { "›" };
    let mut hits = Vec::new();

    for (idx, task) in todos.iter().enumerate() {
        if !show_history && task.created_date < today {
            continue;
        }

        let display_text = if let Some(parent_id) = task.parent {
            let parent_text = todos
                .iter()
                .find(|t| t.id == parent_id)
                .map(|t| t.text.as_str())
                .unwrap_or("");
            format!("{parent_text} {separator} {}", task.text)
        } else {
            task.text.clone()
        };

        let display_chars: Vec<char> = display_text.chars().collect();
        let mut all_matched = true;
        let mut hit_ranges = Vec::new();
        let mut is_prefix_match = false;
        let mut matched_in_text = false;
        let mut matched_in_note = false;
        let mut matched_in_tags = false;

        for token in &tokens {
            match token {
                QueryToken::PriorityMin(min_prio) => {
                    if task.priority < *min_prio {
                        all_matched = false;
                        break;
                    }
                }
                QueryToken::DoneStatus(expected_done) => {
                    if task.done != *expected_done {
                        all_matched = false;
                        break;
                    }
                }
                QueryToken::Tag(tag_prefix) => {
                    let has_matching_tag = task
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().starts_with(tag_prefix));
                    if has_matching_tag {
                        matched_in_tags = true;
                        is_prefix_match = true;
                    } else {
                        all_matched = false;
                        break;
                    }
                }
                QueryToken::Plain(word) => {
                    let text_ranges = find_char_ranges(&display_text, word);
                    let mut token_matched = false;

                    if !text_ranges.is_empty() {
                        token_matched = true;
                        matched_in_text = true;
                        for &(s, _) in &text_ranges {
                            if is_boundary_match(&display_chars, s) {
                                is_prefix_match = true;
                            }
                        }
                        hit_ranges.extend(text_ranges);
                    }

                    // Check tags
                    let word_lower = word.to_lowercase();
                    if task
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&word_lower))
                    {
                        token_matched = true;
                        matched_in_tags = true;
                        if task
                            .tags
                            .iter()
                            .any(|t| t.to_lowercase().starts_with(&word_lower))
                        {
                            is_prefix_match = true;
                        }
                    }

                    // Check note
                    if let Some(ref note) = task.note {
                        if !find_char_ranges(note, word).is_empty() {
                            token_matched = true;
                            matched_in_note = true;
                        }
                    }

                    if !token_matched {
                        all_matched = false;
                        break;
                    }
                }
            }
        }

        if all_matched {
            let has_plain_query = tokens.iter().any(|t| matches!(t, QueryToken::Plain(_)));
            let match_in_note_only =
                has_plain_query && matched_in_note && !matched_in_text && !matched_in_tags;

            hits.push(SearchHit {
                task_id: task.id,
                date: task.created_date,
                is_subtask: task.parent.is_some(),
                parent_id: task.parent,
                display_text,
                done: task.done,
                priority: task.priority,
                tags: task.tags.clone(),
                has_note: task.note.is_some(),
                match_in_note_only,
                is_word_prefix_match: is_prefix_match,
                highlight_ranges: merge_ranges(hit_ranges),
                original_index: idx,
            });
        }
    }

    // Deterministic ranking:
    // 1. word-prefix match before other substring match
    // 2. newest date first
    // 3. manual order within a day (original_index ascending)
    hits.sort_by(|a, b| {
        b.is_word_prefix_match
            .cmp(&a.is_word_prefix_match)
            .then_with(|| b.date.cmp(&a.date))
            .then_with(|| a.original_index.cmp(&b.original_index))
    });

    let total_hits = hits.len();
    if hits.len() > 200 {
        hits.truncate(200);
    }

    SearchResults { hits, total_hits }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query() {
        let q = parse_query("buy milk ! #work is:open unknown:word !! is:done");
        assert_eq!(
            q,
            vec![
                QueryToken::Plain("buy".to_string()),
                QueryToken::Plain("milk".to_string()),
                QueryToken::PriorityMin(1),
                QueryToken::Tag("work".to_string()),
                QueryToken::DoneStatus(false),
                QueryToken::Plain("unknown:word".to_string()),
                QueryToken::PriorityMin(2),
                QueryToken::DoneStatus(true),
            ]
        );
    }

    #[test]
    fn test_and_semantics_all_words_must_match() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let todos = vec![
            Todo {
                id: 1,
                text: "Buy groceries and coffee".to_string(),
                created_date: today,
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Buy groceries only".to_string(),
                created_date: today,
                ..Default::default()
            },
        ];

        let res = search_tasks(&todos, "buy coffee", today, true, false);
        assert_eq!(res.hits.len(), 1);
        assert_eq!(res.hits[0].task_id, 1);
    }

    #[test]
    fn test_priority_and_done_filters() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let todos = vec![
            Todo {
                id: 1,
                text: "Urgent done".to_string(),
                priority: 2,
                done: true,
                created_date: today,
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Urgent open".to_string(),
                priority: 2,
                done: false,
                created_date: today,
                ..Default::default()
            },
            Todo {
                id: 3,
                text: "Normal open".to_string(),
                priority: 0,
                done: false,
                created_date: today,
                ..Default::default()
            },
        ];

        let res_prio = search_tasks(&todos, "!!", today, true, false);
        assert_eq!(res_prio.hits.len(), 2);

        let res_done = search_tasks(&todos, "!! is:done", today, true, false);
        assert_eq!(res_done.hits.len(), 1);
        assert_eq!(res_done.hits[0].task_id, 1);

        let res_open = search_tasks(&todos, "is:open", today, true, false);
        assert_eq!(res_open.hits.len(), 2);
    }

    #[test]
    fn test_tag_search_and_prefix() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let todos = vec![
            Todo {
                id: 1,
                text: "Task with tag".to_string(),
                tags: vec!["working".to_string()],
                created_date: today,
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Task without tag".to_string(),
                tags: vec!["home".to_string()],
                created_date: today,
                ..Default::default()
            },
        ];

        let res = search_tasks(&todos, "#work", today, true, false);
        assert_eq!(res.hits.len(), 1);
        assert_eq!(res.hits[0].task_id, 1);
    }

    #[test]
    fn test_match_in_note_and_marker() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let todos = vec![
            Todo {
                id: 1,
                text: "Meeting".to_string(),
                note: Some("Discuss quarterly budget".to_string()),
                created_date: today,
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Budget planning".to_string(),
                note: Some("Quarterly figures".to_string()),
                created_date: today,
                ..Default::default()
            },
        ];

        let res = search_tasks(&todos, "budget", today, true, false);
        assert_eq!(res.hits.len(), 2);
        let hit1 = res.hits.iter().find(|h| h.task_id == 1).unwrap();
        let hit2 = res.hits.iter().find(|h| h.task_id == 2).unwrap();

        assert!(hit1.match_in_note_only);
        assert!(!hit2.match_in_note_only);
    }

    #[test]
    fn test_subtask_with_parent_context() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let todos = vec![
            Todo {
                id: 1,
                text: "Parent project".to_string(),
                created_date: today,
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Child step".to_string(),
                parent: Some(1),
                created_date: today,
                ..Default::default()
            },
        ];

        let res = search_tasks(&todos, "child", today, true, false);
        assert_eq!(res.hits.len(), 1);
        assert_eq!(res.hits[0].task_id, 2);
        assert_eq!(res.hits[0].display_text, "Parent project › Child step");

        let res_ascii = search_tasks(&todos, "child", today, true, true);
        assert_eq!(
            res_ascii.hits[0].display_text,
            "Parent project > Child step"
        );
    }

    #[test]
    fn test_unicode_accents_emoji_and_combining_marks() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let todos = vec![
            Todo {
                id: 1,
                text: "Café review in Zürich".to_string(),
                created_date: today,
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Ship 🦀 release".to_string(),
                created_date: today,
                ..Default::default()
            },
            Todo {
                id: 3,
                text: "Re\u{0301}sume\u{0301} review".to_string(), // combining acute marks
                created_date: today,
                ..Default::default()
            },
        ];

        // Accented search
        let res_cafe = search_tasks(&todos, "café", today, true, false);
        assert_eq!(res_cafe.hits.len(), 1);
        assert_eq!(res_cafe.hits[0].task_id, 1);
        assert_eq!(res_cafe.hits[0].highlight_ranges, vec![(0, 4)]);

        // Emoji search
        let res_crab = search_tasks(&todos, "🦀", today, true, false);
        assert_eq!(res_crab.hits.len(), 1);
        assert_eq!(res_crab.hits[0].task_id, 2);
        assert_eq!(res_crab.hits[0].highlight_ranges, vec![(5, 6)]);

        // Combining mark search
        let res_comb = search_tasks(&todos, "re\u{0301}sume\u{0301}", today, true, false);
        assert_eq!(res_comb.hits.len(), 1);
        assert_eq!(res_comb.hits[0].task_id, 3);
    }

    #[test]
    fn test_ranking_prefix_over_substring_and_newest_date_first() {
        let d_old = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        let d_new = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let todos = vec![
            Todo {
                id: 1,
                text: "Subtask item".to_string(), // "task" is NOT prefix of "Subtask"
                created_date: d_new,
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Task item old".to_string(), // "task" IS prefix
                created_date: d_old,
                ..Default::default()
            },
            Todo {
                id: 3,
                text: "Task item new".to_string(), // "task" IS prefix
                created_date: d_new,
                ..Default::default()
            },
        ];

        let res = search_tasks(&todos, "task", d_new, true, false);
        assert_eq!(res.hits.len(), 3);
        // Prefix matches (3 and 2) before substring (1)
        // Between prefix matches: newer date (3) before older date (2)
        assert_eq!(res.hits[0].task_id, 3);
        assert_eq!(res.hits[1].task_id, 2);
        assert_eq!(res.hits[2].task_id, 1);
    }

    #[test]
    fn test_cap_at_200_hits() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut todos = Vec::new();
        for i in 1..=250 {
            todos.push(Todo {
                id: i,
                text: format!("Task number {i}"),
                created_date: today,
                ..Default::default()
            });
        }

        let res = search_tasks(&todos, "task", today, true, false);
        assert_eq!(res.hits.len(), 200);
        assert_eq!(res.total_hits, 250);
    }

    #[test]
    fn test_show_history_false_hides_past_hits() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let past = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
        let todos = vec![
            Todo {
                id: 1,
                text: "Past task".to_string(),
                created_date: past,
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Today task".to_string(),
                created_date: today,
                ..Default::default()
            },
        ];

        // show_history = false -> hides past
        let res_no_hist = search_tasks(&todos, "task", today, false, false);
        assert_eq!(res_no_hist.hits.len(), 1);
        assert_eq!(res_no_hist.hits[0].task_id, 2);

        // show_history = true -> shows past
        let res_with_hist = search_tasks(&todos, "task", today, true, false);
        assert_eq!(res_with_hist.hits.len(), 2);
    }
}
