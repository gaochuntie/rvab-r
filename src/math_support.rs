use std::cmp::Ordering;

pub struct Interval{
    start:u64,
    end:u64,
}
impl Interval {
    pub fn new(start:u64,end:u64) -> Self {
        Interval{start,end}
    }
    pub fn get_start(&self) -> u64 {
        self.start
    }
    pub fn get_end(&self) -> u64 {
        self.end
    }
}
impl Ord for Interval {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start.cmp(&other.start)
    }
}
impl PartialOrd for Interval {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Interval {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}

impl Eq for Interval {}

#[derive(PartialEq)]
pub enum IntervalState {
    Disjoint,
    Equal,
    Subset,
    Superset,
    Overlap,
}
/// check states of two closed intervals
/// If the first interval (interval1) is a subset of the second interval (interval2),
/// the function will return IntervalState::Subset.
pub fn check_interval_state(interval1:&Interval,interval2:&Interval) -> IntervalState {
    if interval1.start<interval2.end || interval1.start>interval2.end {
        return IntervalState::Disjoint;
    }
    if interval1.start==interval2.start && interval1.end==interval2.end {
        return IntervalState::Equal;
    }
    if interval1.start>=interval2.start && interval1.end<=interval2.end {
        return IntervalState::Subset;
    }
    if interval1.start<=interval2.start && interval1.end>=interval2.end {
        return IntervalState::Superset;
    }
    IntervalState::Overlap
}

/// check if any intervals in the vector overlap
/// If any two intervals overlap, the function will return true.
pub fn check_any_overlaps(intervals:&mut Vec<Interval>) -> bool {
    // Sort intervals by their start points
    let mut intervals = intervals;
    intervals.sort();
    // Check for overlap
    for i in 1..intervals.len() {
        let prev = &intervals[i - 1];
        let current = &intervals[i];
        if current.start <= prev.end {
            // Intervals overlap
            return true;
        }
    }
    // No overlap found
    false
}