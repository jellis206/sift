use std::cmp::Ordering;
use std::collections::BinaryHeap;

use chrono::{DateTime, Utc};
use clap::ValueEnum;

use crate::sources::Prompt;

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Order {
    Newest,
    Oldest,
}

/// Keeps the first `limit` prompts in `order` using O(limit) memory.
/// Prompts without a timestamp rank as oldest; equal times keep scan order.
// ponytail: unlimited sorts hold every match in memory; spill sorted chunks
// to disk if result sets ever outgrow RAM.
pub struct TopK {
    heap: BinaryHeap<Ranked>,
    limit: usize,
    newest: bool,
    seq: usize,
}

impl TopK {
    pub fn new(order: Order, limit: Option<usize>) -> Self {
        TopK {
            heap: BinaryHeap::new(),
            limit: limit.unwrap_or(usize::MAX),
            newest: order == Order::Newest,
            seq: 0,
        }
    }

    pub fn push(&mut self, prompt: Prompt) {
        self.heap.push(Ranked {
            time: prompt.timestamp,
            seq: self.seq,
            newest: self.newest,
            prompt,
        });
        self.seq += 1;
        // The max is the worst-ranked prompt kept so far.
        if self.heap.len() > self.limit {
            self.heap.pop();
        }
    }

    /// Best-ranked first.
    pub fn into_sorted(self) -> Vec<Prompt> {
        let ranked = self.heap.into_sorted_vec();
        ranked.into_iter().map(|r| r.prompt).collect()
    }
}

/// Orders better-ranked prompts first: `Less` means shown earlier.
struct Ranked {
    time: Option<DateTime<Utc>>,
    seq: usize,
    newest: bool,
    prompt: Prompt,
}

impl Ord for Ranked {
    fn cmp(&self, other: &Self) -> Ordering {
        let by_time = self.time.cmp(&other.time);
        let by_time = if self.newest {
            by_time.reverse()
        } else {
            by_time
        };
        by_time.then(self.seq.cmp(&other.seq))
    }
}

impl PartialOrd for Ranked {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Ranked {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Ranked {}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;
    use crate::sources::Source;

    fn prompt(text: &str, secs: Option<i64>) -> Prompt {
        Prompt {
            source: Source::Claude,
            timestamp: secs.and_then(|s| DateTime::from_timestamp(s, 0)),
            session_id: None,
            text: text.to_string(),
        }
    }

    fn texts(order: Order, limit: Option<usize>) -> Vec<String> {
        let mut top = TopK::new(order, limit);
        for (text, secs) in [
            ("b", Some(2)),
            ("none", None),
            ("c", Some(3)),
            ("a", Some(1)),
            ("c2", Some(3)),
        ] {
            top.push(prompt(text, secs));
        }
        top.into_sorted().into_iter().map(|p| p.text).collect()
    }

    #[test]
    fn newest_first_untimed_last_ties_in_scan_order() {
        assert_eq!(texts(Order::Newest, None), ["c", "c2", "b", "a", "none"]);
    }

    #[test]
    fn oldest_first_untimed_first() {
        assert_eq!(texts(Order::Oldest, None), ["none", "a", "b", "c", "c2"]);
    }

    #[test]
    fn limit_keeps_best_ranked() {
        assert_eq!(texts(Order::Newest, Some(2)), ["c", "c2"]);
        assert_eq!(texts(Order::Oldest, Some(2)), ["none", "a"]);
        assert!(texts(Order::Newest, Some(0)).is_empty());
    }
}
