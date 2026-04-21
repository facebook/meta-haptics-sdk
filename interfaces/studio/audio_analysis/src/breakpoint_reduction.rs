// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

//! # Breakpoint Reduction
//!
//! This module provides an algorithm for reducing the number of breakpoints in haptic signals,
//! without distorting the general shape of the 'curve'.
//!
//! ## Implementation background
//!
//! The approach taken here is an adaptation of the Visvalingam-Whyatt line simplification,
//! described nicely [here][1], taking a starting point from [this Rust implementation][2].
//!
//! This implementation was also the starting point for the version of the algorithm found in the
//! [geo crate][3].
//!
//! Why aren't we using the geo crate instead of reimplementing the VW algorithm for the Nth time?
//!   - We want to reduce the curve until we hit a target number of breakpoints, rather than
//!     reducing all triangles past an area threshold. The geo crate doesn't provide that feature.
//!   - It's nice to allow different breakpoint types to hold on to their own representation
//!     (with meaningful member names) rather than impose a fixed 'x/y' layout.
//!   - Having our own implementation also allows us to customize the behaviour based on our needs.
//!
//! ## Algorithm overview
//!
//! The algorithm as implemented here follows these steps:
//! - For a given input signal, separate the signal into regions by time.
//! - For each region, apply VW reduction, until a target number of breakpoints is met.
//! - As a follow up step, remove triangles with smaller scores than a defined minimum.
//!   - This avoids silent regions with lots of redundant breakpoints.
//! - The reduced regions are then collected into a resulting reduced signal.
//!
//! [1]: https://bost.ocks.org/mike/simplify
//! [2]: https://github.com/huonw/isrustfastyet/blob/bb518a4fa46d77293154e2334d6e14fbca44d3c0/mem/line_simplify.rs
//! [3]: https://docs.rs/geo/0.15.0/geo/algorithm/simplifyvw/trait.SimplifyVW.html

use std::cmp::Ordering;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// A trait for breakpoints, allowing time/value structs to maintain their own representations
pub trait Breakpoint {
    fn time(&self) -> f32;
    fn value(&self) -> f32;
}

/// The settings used by the breakpoint reduction algorithm
///
/// See [reduce_breakpoints]
pub struct ReduceBreakpointsSettings {
    /// The duration of each analysis region in time (i.e. the X axis).
    /// The input signal will be broken up into regions of this duration,
    /// with each region reduced independently.
    pub region_duration: f32,
    /// If there are fewer breakpoints in the region then all of them will be kept as a starting
    /// point, before the minimum score is applied.
    pub maximum_breakpoints_per_region: usize,
    /// The minimum score that a breakpoint should have to be kept in the resulting output.
    /// This is applied after analysis of the complete signal,
    /// so it may reduce the number of resulting breakpoints further below the
    /// maximum_breakpoints_per_region value.
    pub minimum_score: f32,
}

impl Default for ReduceBreakpointsSettings {
    fn default() -> Self {
        Self {
            region_duration: 1.0,
            maximum_breakpoints_per_region: 50,
            minimum_score: 0.0,
        }
    }
}

// A struct for keeping track of triangles, used by [reduce_breakpoints_for_region]
#[derive(Debug, PartialOrd, PartialEq)]
struct TriangleScore {
    score: f32,
    left_index: usize,
    mid_index: usize,
    right_index: usize,
}

// Clippy doesn't like PartialOrd being derived while Ord is explicitly implemented,
// and we can't derive Ord with an f32, but given that we won't be encountering NaNs or infinity
// (hence the unwrap of the partial_cmp result), we can advise Clippy that it doesn't need to worry.
#[allow(clippy::derive_ord_xor_partial_ord)]
// Ord is required for use in BinaryHeap
impl Ord for TriangleScore {
    fn cmp(&self, other: &TriangleScore) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
// Eq is required by Ord
impl Eq for TriangleScore {}

fn triangle_score<T>(left: &T, mid: &T, right: &T) -> f32
where
    T: Breakpoint + Clone,
{
    let double_area = (left.time() - mid.time()) * (right.value() - mid.value())
        - (right.time() - mid.time()) * (left.value() - mid.value());
    double_area.abs()
}

// Performs breakpoint reduction for a region of breakpoints
// - See [reduce_breakpoints] for the public entry point
// - See the module documentation for an overview of the algorithm.
fn reduce_breakpoints_for_region<T>(
    result: &mut Vec<T>,
    breakpoints: &[T],
    maximum_breakpoints: usize,
    min_score: f32,
) where
    T: Breakpoint + Clone,
{
    // Simulate a linked list by making previous/next indices for each breakpoint.
    //   breakpoints.len() => at edge
    //   (0, 0) => point is removed
    let at_edge = breakpoints.len();
    let discarded = (0, 0);
    let mut reduced_breakpoints: Vec<(usize, usize)> = (0..breakpoints.len())
        .map(|i| if i == 0 { (at_edge, 1) } else { (i - 1, i + 1) })
        .collect();

    // Collect the triangle scores into a binary heap.
    // Note that by default BinaryHeap is a max-heap, so we wrap the scores in cmp::Reverse to
    // reverse the heap ordering, allowing us to easily pop the smallest triangle.
    let mut heap: BinaryHeap<Reverse<TriangleScore>> = breakpoints
        .windows(3) // Iterate over each overlapping group of 3 breakpoints
        .enumerate() // Include the index of the first breakpoint in the triangle
        .map(|(i, triangle)| match triangle {
            [left, mid, right] => Reverse(TriangleScore {
                score: triangle_score(left, mid, right),
                left_index: i,
                mid_index: i + 1,
                right_index: i + 2,
            }),
            _ => unreachable!(),
        })
        .collect();

    // Keep removing breakpoints until we hit our target count
    // Minus 2 for the start and end breakpoints, which are always kept
    let mut result_count = breakpoints.len();
    while result_count > maximum_breakpoints {
        let Reverse(smallest) = match heap.pop() {
            Some(smallest) => smallest,
            None => break,
        };

        let mid_index = smallest.mid_index;
        let (left_index, right_index) = reduced_breakpoints[mid_index];

        if left_index != smallest.left_index || right_index != smallest.right_index {
            // A point in the triangle was removed during a previous iteration, so we can move on.
            continue;
        }

        // Remove the middle breakpoint from the reduced_breakpoints list by adjusting the
        // neighbouring triangle indices, thereby creating two new triangles.
        // The old now-redundant triangles to the left and right will be popped from the heap
        // and ignored on later iterations.
        debug_assert_eq!(reduced_breakpoints[left_index].1, mid_index);
        debug_assert_eq!(reduced_breakpoints[right_index].0, mid_index);

        reduced_breakpoints[left_index].1 = right_index;
        reduced_breakpoints[mid_index] = discarded;
        reduced_breakpoints[right_index].0 = left_index;
        result_count -= 1;

        // Add scores for the new triangles to the heap
        for &mid_new in [left_index, right_index].iter() {
            let (left_new, right_new) = reduced_breakpoints[mid_new];

            if left_new == at_edge || right_new == at_edge {
                // No need to calculate a new score for an edge triangle
                continue;
            }

            let new_triangle_score = triangle_score(
                &breakpoints[left_new],
                &breakpoints[mid_new],
                &breakpoints[right_new],
            );

            heap.push(Reverse(TriangleScore {
                score: new_triangle_score,
                left_index: left_new,
                mid_index: mid_new,
                right_index: right_new,
            }));
        }
    }

    // Keep removing breakpoints while their score is less than the specified minimum
    while let Some(Reverse(smallest)) = heap.peek() {
        if smallest.score >= min_score {
            // If the smallest score in the heap is above the threshold then break
            break;
        }

        reduced_breakpoints[smallest.mid_index] = discarded;
        heap.pop();
    }

    // Extend the result with breakpoints that are still valid in the reduced_breakpoints list
    let reduced_breakpoints_for_region = breakpoints
        .iter()
        .zip(reduced_breakpoints.iter())
        .filter_map(|(breakpoint, indices)| {
            if *indices != discarded {
                Some(breakpoint.clone())
            } else {
                None
            }
        });

    // The result is a pre-allocated mutable Vec,
    // so we can extend it with the filtered reduced breakpoints.
    result.extend(reduced_breakpoints_for_region);
}

/// Reduces a series of breakpoints, keeping the most significant.
///
/// See the module documentation for a description of the algorithm.
pub fn reduce_breakpoints<T>(breakpoints: &[T], settings: ReduceBreakpointsSettings) -> Vec<T>
where
    T: Breakpoint + Clone,
{
    // Pre-allocate a Vec with an estimated capacity,
    // avoiding lots of reallocations as the result is calculated.
    let estimated_result_size = breakpoints.last().map_or(0, |last_breakpoint| {
        (last_breakpoint.time() / settings.region_duration
            * settings.maximum_breakpoints_per_region as f32) as usize
    });
    let mut result = Vec::with_capacity(estimated_result_size);

    // Iterate over each consecutive time region of the breakpoints,
    // and reduce each region's breakpoints separately.
    let mut start_index = 0;
    let start_time = breakpoints
        .first()
        .map_or(0.0, |first_breakpoint| first_breakpoint.time());
    let mut region_end_time = start_time + settings.region_duration;

    while start_index < breakpoints.len() {
        let end_index = match breakpoints[start_index..]
            .iter()
            .position(|breakpoint| breakpoint.time() >= region_end_time)
        {
            Some(end_offset) => start_index + end_offset,
            None => breakpoints.len(),
        };

        reduce_breakpoints_for_region(
            &mut result,
            &breakpoints[start_index..end_index],
            settings.maximum_breakpoints_per_region,
            settings.minimum_score,
        );

        region_end_time += settings.region_duration;
        start_index = end_index;
    }

    result
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use super::*;

    #[derive(Clone, PartialEq)]
    struct TestBreakpoint {
        time: f32,
        value: f32,
    }

    impl Breakpoint for TestBreakpoint {
        fn time(&self) -> f32 {
            self.time
        }

        fn value(&self) -> f32 {
            self.value
        }
    }

    impl From<(f32, f32)> for TestBreakpoint {
        fn from((time, value): (f32, f32)) -> Self {
            Self { time, value }
        }
    }

    impl fmt::Debug for TestBreakpoint {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "({}, {})", self.time, self.value)
        }
    }

    fn check_reduction(signal: &[((f32, f32), bool)], settings: ReduceBreakpointsSettings) {
        let input: Vec<TestBreakpoint> = signal
            .iter()
            .map(|((time, value), _)| TestBreakpoint {
                time: *time,
                value: *value,
            })
            .collect();
        let expected: Vec<TestBreakpoint> = signal
            .iter()
            .filter(|(_, keep)| *keep)
            .map(|((time, value), _)| TestBreakpoint {
                time: *time,
                value: *value,
            })
            .collect();
        let output = reduce_breakpoints(&input, settings);

        for (i, (expected_bp, actual_bp)) in expected.iter().zip(output.iter()).enumerate() {
            assert_eq!(
                expected_bp, actual_bp,
                "\nMismatch at position {i} - expected: {expected_bp:?}, actual: {actual_bp:?}\
                 \n  expected output: {expected:?}\
                 \n  actual output: {output:?}\n",
            );
        }
    }

    #[test]
    fn reduction_no_minimum_score() {
        let keep = true;
        let signal = vec![
            // Start with a flat line at 0.0
            ((0.0, 0.0), keep),
            ((0.1, 0.0), !keep),
            ((0.2, 0.0), keep),
            // Linear ramp up to 1.0
            ((0.3, 0.25), !keep),
            ((0.4, 0.5), !keep),
            ((0.5, 0.75), !keep),
            // Hold at 1.0
            ((0.6, 1.0), keep),
            ((0.7, 1.0), keep),
            // Ramp down to 0.5 and back up to 1.0
            ((0.8, 0.5), keep),
            ((0.9, 1.0), keep),
        ];
        let settings = ReduceBreakpointsSettings {
            region_duration: 1.0,
            maximum_breakpoints_per_region: 6,
            minimum_score: 0.0,
        };

        check_reduction(&signal, settings)
    }

    #[test]
    fn reduction_remove_least_significant_no_minimum_score() {
        let keep = true;
        let signal = vec![
            // Start with a flat line at 0.0
            ((0.0, 0.0), keep),
            ((0.1, 0.0), !keep),
            ((0.2, 0.0), keep),
            // Linear ramp up to 1.0
            ((0.3, 0.25), !keep),
            ((0.4, 0.5), !keep),
            ((0.5, 0.75), !keep),
            // Hold at 1.0
            ((0.6, 1.0), keep),
            ((0.7, 1.0), !keep),
            // Ramp down to 0.5 and back up to 1.0
            ((0.8, 0.5), !keep),
            ((0.9, 1.0), keep),
        ];

        check_reduction(
            &signal,
            ReduceBreakpointsSettings {
                region_duration: 1.0,
                maximum_breakpoints_per_region: 4,
                minimum_score: 0.0,
            },
        )
    }

    #[test]
    fn reduction_remove_via_minimum_score() {
        let keep = true;
        let signal = vec![
            // Start with a flat line at 0.0
            ((0.0, 0.0), keep),
            ((0.1, 0.0), !keep),
            ((0.2, 0.0), keep),
            // Linear ramp up to 1.0
            ((0.3, 0.25), !keep),
            ((0.4, 0.5), !keep),
            ((0.5, 0.75), !keep),
            // Hold at 1.0
            ((0.6, 1.0), keep),
            ((0.7, 1.0), keep),
            // Ramp down to 0.5 and back up to 1.0
            ((0.8, 0.5), keep),
            ((0.9, 1.0), keep),
        ];

        check_reduction(
            &signal,
            ReduceBreakpointsSettings {
                region_duration: 1.0,
                maximum_breakpoints_per_region: 10,
                minimum_score: 0.01,
            },
        )
    }
}
