//! App state

use crate::model::{Diff, FileDiff, Hunk, ReviewStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Help,
    ConfirmQuit,
    PendingG,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndoEntry {
    pub file_index: usize,
    pub hunk_index: usize,
    pub old_status: ReviewStatus,
}

pub struct AppState {
    pub diff: Diff,
    pub file_index: usize,
    pub hunk_index: usize,
    pub mode: AppMode,
    pub should_quit: bool,
    pub undo_stack: Vec<UndoEntry>,
    pub viewport_offset: usize,
    pub viewport_height: usize,
}

impl AppState {
    pub fn new(diff: Diff) -> Self {
        Self {
            diff,
            file_index: 0,
            hunk_index: 0,
            mode: AppMode::Normal,
            should_quit: false,
            undo_stack: Vec::new(),
            viewport_offset: 0,
            viewport_height: 24,
        }
    }

    pub fn current_file(&self) -> Option<&FileDiff> {
        self.diff.files.get(self.file_index)
    }

    pub fn current_hunk(&self) -> Option<&Hunk> {
        self.current_file()
            .and_then(|f| f.hunks.get(self.hunk_index))
    }

    pub fn current_hunk_mut(&mut self) -> Option<&mut Hunk> {
        self.diff
            .files
            .get_mut(self.file_index)
            .and_then(|f| f.hunks.get_mut(self.hunk_index))
    }

    pub fn next_hunk(&mut self) {
        if let Some(file) = self.current_file() {
            if self.hunk_index + 1 < file.hunks.len() {
                self.hunk_index += 1;
            } else if self.file_index + 1 < self.diff.files.len() {
                self.file_index += 1;
                self.hunk_index = 0;
                self.viewport_offset = 0;
            }
        }
        self.ensure_visible();
    }

    pub fn prev_hunk(&mut self) {
        if self.hunk_index > 0 {
            self.hunk_index -= 1;
        } else if self.file_index > 0 {
            self.file_index -= 1;
            if let Some(file) = self.current_file() {
                self.hunk_index = file.hunks.len().saturating_sub(1);
            }
            self.viewport_offset = 0;
        }
        self.ensure_visible();
    }

    pub fn next_file(&mut self) {
        if self.file_index + 1 < self.diff.files.len() {
            self.file_index += 1;
            self.hunk_index = 0;
            self.viewport_offset = 0;
        }
        self.ensure_visible();
    }

    pub fn prev_file(&mut self) {
        if self.file_index > 0 {
            self.file_index -= 1;
            self.hunk_index = 0;
            self.viewport_offset = 0;
        }
        self.ensure_visible();
    }

    fn push_undo(&mut self, file_index: usize, hunk_index: usize, old_status: ReviewStatus) {
        self.undo_stack.push(UndoEntry {
            file_index,
            hunk_index,
            old_status,
        });
    }

    pub fn set_current_status(&mut self, status: ReviewStatus) {
        if let Some(hunk) = self.current_hunk() {
            let old_status = hunk.status;
            let fi = self.file_index;
            let hi = self.hunk_index;
            self.push_undo(fi, hi, old_status);
        }
        if let Some(hunk) = self.current_hunk_mut() {
            hunk.status = status;
        }
    }

    pub fn toggle_current_status(&mut self) {
        if let Some(hunk) = self.current_hunk() {
            let old_status = hunk.status;
            let fi = self.file_index;
            let hi = self.hunk_index;
            self.push_undo(fi, hi, old_status);
        }
        if let Some(hunk) = self.current_hunk_mut() {
            hunk.status = match hunk.status {
                ReviewStatus::Pending => ReviewStatus::Accepted,
                ReviewStatus::Accepted => ReviewStatus::Rejected,
                ReviewStatus::Rejected => ReviewStatus::Pending,
            };
        }
    }

    pub fn set_all_status(&mut self, status: ReviewStatus) {
        for fi in 0..self.diff.files.len() {
            for hi in 0..self.diff.files[fi].hunks.len() {
                let old_status = self.diff.files[fi].hunks[hi].status;
                self.push_undo(fi, hi, old_status);
                self.diff.files[fi].hunks[hi].status = status;
            }
        }
    }

    pub fn undo(&mut self) {
        if let Some(entry) = self.undo_stack.pop() {
            let old_fi = self.file_index;
            if let Some(file) = self.diff.files.get_mut(entry.file_index)
                && let Some(hunk) = file.hunks.get_mut(entry.hunk_index)
            {
                hunk.status = entry.old_status;
            }
            self.file_index = entry.file_index;
            self.hunk_index = entry.hunk_index;
            if entry.file_index != old_fi {
                self.viewport_offset = 0;
            }
            self.ensure_visible();
        }
    }

    pub fn first_hunk(&mut self) {
        self.file_index = 0;
        self.hunk_index = 0;
        self.viewport_offset = 0;
        self.ensure_visible();
    }

    pub fn last_hunk(&mut self) {
        if let Some(last_fi) = self.diff.files.len().checked_sub(1) {
            self.file_index = last_fi;
            self.hunk_index = self.diff.files[last_fi].hunks.len().saturating_sub(1);
            self.viewport_offset = 0;
            self.ensure_visible();
        }
    }

    /// Move to the next Pending hunk (wrap-around). Returns false if none found.
    pub fn next_pending(&mut self) -> bool {
        let total = self.total_hunks();
        if total == 0 {
            return false;
        }
        let start = self.flat_hunk_index();
        let old_fi = self.file_index;
        for offset in 1..=total {
            let flat = (start + offset) % total;
            let (fi, hi) = self.flat_to_indices(flat);
            if self.diff.files[fi].hunks[hi].status == ReviewStatus::Pending {
                self.file_index = fi;
                self.hunk_index = hi;
                if fi != old_fi {
                    self.viewport_offset = 0;
                }
                self.ensure_visible();
                return true;
            }
        }
        false
    }

    pub fn flat_to_indices(&self, flat: usize) -> (usize, usize) {
        let mut remaining = flat;
        for (fi, file) in self.diff.files.iter().enumerate() {
            if remaining < file.hunks.len() {
                return (fi, remaining);
            }
            remaining -= file.hunks.len();
        }
        // fallback: last hunk
        let last_fi = self.diff.files.len().saturating_sub(1);
        let last_hi = self.diff.files.get(last_fi).map_or(0, |f| f.hunks.len().saturating_sub(1));
        (last_fi, last_hi)
    }

    pub fn total_hunks(&self) -> usize {
        self.diff.files.iter().map(|f| f.hunks.len()).sum()
    }

    pub fn reviewed_hunks(&self) -> usize {
        self.diff
            .files
            .iter()
            .flat_map(|f| &f.hunks)
            .filter(|h| h.status != ReviewStatus::Pending)
            .count()
    }

    pub fn accepted_hunks(&self) -> usize {
        self.diff
            .files
            .iter()
            .flat_map(|f| &f.hunks)
            .filter(|h| h.status == ReviewStatus::Accepted)
            .count()
    }

    // --- Viewport / scroll ---

    pub fn current_hunk_line_offset(&self) -> usize {
        let file = match self.current_file() {
            Some(f) => f,
            None => return 0,
        };
        let mut offset = 0;
        for (hi, hunk) in file.hunks.iter().enumerate() {
            if hi == self.hunk_index {
                return offset;
            }
            // All non-current hunks are collapsed (1 line header only)
            offset += 1;
            let _ = hunk; // suppress unused warning
        }
        offset
    }

    pub fn virtual_doc_height(&self) -> usize {
        let file = match self.current_file() {
            Some(f) => f,
            None => return 0,
        };
        let mut height = 0;
        for (hi, hunk) in file.hunks.iter().enumerate() {
            if hi == self.hunk_index {
                height += 1 + hunk.lines.len();
            } else {
                height += 1;
            }
        }
        height
    }

    pub fn ensure_visible(&mut self) {
        let offset = self.current_hunk_line_offset();
        let current_hunk_height = self.current_hunk().map_or(1, |h| 1 + h.lines.len());

        // If current hunk starts above viewport, scroll up
        if offset < self.viewport_offset {
            self.viewport_offset = offset;
        }
        // If current hunk ends below viewport, scroll down
        let bottom = offset + current_hunk_height;
        if bottom > self.viewport_offset + self.viewport_height {
            self.viewport_offset = bottom.saturating_sub(self.viewport_height);
        }
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.viewport_offset = self.viewport_offset.saturating_sub(n);
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = self.virtual_doc_height().saturating_sub(self.viewport_height);
        self.viewport_offset = (self.viewport_offset + n).min(max);
    }

    pub fn flat_hunk_index(&self) -> usize {
        let mut index = 0;
        for (fi, file) in self.diff.files.iter().enumerate() {
            if fi < self.file_index {
                index += file.hunks.len();
            } else if fi == self.file_index {
                index += self.hunk_index;
                break;
            }
        }
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DiffLine, FileDiff, Hunk};

    fn make_hunk(status: ReviewStatus) -> Hunk {
        Hunk {
            header: "@@ -1,1 +1,1 @@".to_string(),
            old_start: 1,
            old_count: 1,
            new_start: 1,
            new_count: 1,
            lines: vec![DiffLine::Context("x".to_string())],
            status,
        }
    }

    fn make_file(name: &str, hunks: Vec<Hunk>) -> FileDiff {
        FileDiff {
            old_path: name.to_string(),
            new_path: name.to_string(),
            raw_old_path: format!("a/{}", name),
            raw_new_path: format!("b/{}", name),
            hunks,
            is_binary: false,
        }
    }

    fn make_state(files: Vec<FileDiff>) -> AppState {
        AppState::new(Diff { files })
    }

    // --- Undo tests ---

    #[test]
    fn test_undo_single() {
        let mut state = make_state(vec![make_file("a.rs", vec![make_hunk(ReviewStatus::Pending)])]);
        state.set_current_status(ReviewStatus::Accepted);
        assert_eq!(state.current_hunk().unwrap().status, ReviewStatus::Accepted);
        state.undo();
        assert_eq!(state.current_hunk().unwrap().status, ReviewStatus::Pending);
    }

    #[test]
    fn test_undo_toggle() {
        let mut state = make_state(vec![make_file("a.rs", vec![make_hunk(ReviewStatus::Pending)])]);
        state.toggle_current_status(); // Pending -> Accepted
        assert_eq!(state.current_hunk().unwrap().status, ReviewStatus::Accepted);
        state.undo();
        assert_eq!(state.current_hunk().unwrap().status, ReviewStatus::Pending);
    }

    #[test]
    fn test_undo_empty_stack() {
        let mut state = make_state(vec![make_file("a.rs", vec![make_hunk(ReviewStatus::Pending)])]);
        state.undo(); // should be no-op
        assert_eq!(state.current_hunk().unwrap().status, ReviewStatus::Pending);
        assert_eq!(state.file_index, 0);
        assert_eq!(state.hunk_index, 0);
    }

    #[test]
    fn test_undo_accept_all() {
        let mut state = make_state(vec![
            make_file("a.rs", vec![make_hunk(ReviewStatus::Pending), make_hunk(ReviewStatus::Pending)]),
            make_file("b.rs", vec![make_hunk(ReviewStatus::Pending)]),
        ]);
        state.set_all_status(ReviewStatus::Accepted);
        // 3 undo entries (one per hunk)
        assert_eq!(state.undo_stack.len(), 3);
        // Undo last (b.rs hunk 0)
        state.undo();
        assert_eq!(state.diff.files[1].hunks[0].status, ReviewStatus::Pending);
        assert_eq!(state.file_index, 1);
        assert_eq!(state.hunk_index, 0);
        // Undo second (a.rs hunk 1)
        state.undo();
        assert_eq!(state.diff.files[0].hunks[1].status, ReviewStatus::Pending);
        assert_eq!(state.file_index, 0);
        assert_eq!(state.hunk_index, 1);
        // Undo first (a.rs hunk 0)
        state.undo();
        assert_eq!(state.diff.files[0].hunks[0].status, ReviewStatus::Pending);
    }

    #[test]
    fn test_undo_navigates_to_hunk() {
        let mut state = make_state(vec![
            make_file("a.rs", vec![make_hunk(ReviewStatus::Pending)]),
            make_file("b.rs", vec![make_hunk(ReviewStatus::Pending)]),
        ]);
        // Accept hunk in file 0
        state.set_current_status(ReviewStatus::Accepted);
        // Move to file 1
        state.next_file();
        assert_eq!(state.file_index, 1);
        // Undo should navigate back to file 0
        state.undo();
        assert_eq!(state.file_index, 0);
        assert_eq!(state.hunk_index, 0);
        assert_eq!(state.current_hunk().unwrap().status, ReviewStatus::Pending);
    }

    // --- Vim navigation tests ---

    #[test]
    fn test_first_hunk() {
        let mut state = make_state(vec![
            make_file("a.rs", vec![make_hunk(ReviewStatus::Pending), make_hunk(ReviewStatus::Pending)]),
            make_file("b.rs", vec![make_hunk(ReviewStatus::Pending)]),
        ]);
        state.file_index = 1;
        state.hunk_index = 0;
        state.first_hunk();
        assert_eq!(state.file_index, 0);
        assert_eq!(state.hunk_index, 0);
    }

    #[test]
    fn test_last_hunk() {
        let mut state = make_state(vec![
            make_file("a.rs", vec![make_hunk(ReviewStatus::Pending)]),
            make_file("b.rs", vec![make_hunk(ReviewStatus::Pending), make_hunk(ReviewStatus::Pending)]),
        ]);
        state.last_hunk();
        assert_eq!(state.file_index, 1);
        assert_eq!(state.hunk_index, 1);
    }

    #[test]
    fn test_next_pending_wrap() {
        let mut state = make_state(vec![
            make_file("a.rs", vec![make_hunk(ReviewStatus::Accepted), make_hunk(ReviewStatus::Pending)]),
            make_file("b.rs", vec![make_hunk(ReviewStatus::Accepted)]),
        ]);
        state.file_index = 1;
        state.hunk_index = 0;
        let found = state.next_pending();
        assert!(found);
        // Should wrap to a.rs hunk 1
        assert_eq!(state.file_index, 0);
        assert_eq!(state.hunk_index, 1);
    }

    #[test]
    fn test_next_pending_none() {
        let mut state = make_state(vec![
            make_file("a.rs", vec![make_hunk(ReviewStatus::Accepted)]),
        ]);
        let found = state.next_pending();
        assert!(!found);
        assert_eq!(state.file_index, 0);
        assert_eq!(state.hunk_index, 0);
    }

    #[test]
    fn test_flat_to_indices() {
        let state = make_state(vec![
            make_file("a.rs", vec![make_hunk(ReviewStatus::Pending), make_hunk(ReviewStatus::Pending)]),
            make_file("b.rs", vec![make_hunk(ReviewStatus::Pending)]),
        ]);
        assert_eq!(state.flat_to_indices(0), (0, 0));
        assert_eq!(state.flat_to_indices(1), (0, 1));
        assert_eq!(state.flat_to_indices(2), (1, 0));
    }

    // --- Viewport tests ---

    fn make_hunk_with_lines(n: usize, status: ReviewStatus) -> Hunk {
        Hunk {
            header: "@@ -1,1 +1,1 @@".to_string(),
            old_start: 1,
            old_count: 1,
            new_start: 1,
            new_count: 1,
            lines: (0..n).map(|i| DiffLine::Context(format!("line{}", i))).collect(),
            status,
        }
    }

    #[test]
    fn test_virtual_doc_height() {
        // 3 hunks, current=0 with 5 lines
        let mut state = make_state(vec![make_file("a.rs", vec![
            make_hunk_with_lines(5, ReviewStatus::Pending),
            make_hunk_with_lines(3, ReviewStatus::Pending),
            make_hunk_with_lines(2, ReviewStatus::Pending),
        ])]);
        state.hunk_index = 0;
        // hunk0: 1 header + 5 lines = 6, hunk1: 1, hunk2: 1 => 8
        assert_eq!(state.virtual_doc_height(), 8);
    }

    #[test]
    fn test_ensure_visible_scrolls_down() {
        let mut state = make_state(vec![make_file("a.rs", vec![
            make_hunk_with_lines(10, ReviewStatus::Pending),
            make_hunk_with_lines(10, ReviewStatus::Pending),
            make_hunk_with_lines(10, ReviewStatus::Pending),
        ])]);
        state.viewport_height = 5;
        state.viewport_offset = 0;
        state.hunk_index = 2;
        state.ensure_visible();
        // hunk2 offset = 2 (collapsed hunk0=1 + collapsed hunk1=1)
        // hunk2 height = 1 + 10 = 11, bottom = 13
        // viewport_offset should be 13 - 5 = 8
        assert_eq!(state.viewport_offset, 8);
    }

    #[test]
    fn test_ensure_visible_scrolls_up() {
        let mut state = make_state(vec![make_file("a.rs", vec![
            make_hunk_with_lines(5, ReviewStatus::Pending),
            make_hunk_with_lines(5, ReviewStatus::Pending),
        ])]);
        state.viewport_height = 10;
        state.viewport_offset = 5;
        state.hunk_index = 0;
        state.ensure_visible();
        assert_eq!(state.viewport_offset, 0);
    }

    #[test]
    fn test_page_down_clamps() {
        let mut state = make_state(vec![make_file("a.rs", vec![
            make_hunk_with_lines(3, ReviewStatus::Pending),
        ])]);
        state.viewport_height = 10;
        state.viewport_offset = 0;
        state.scroll_down(100);
        // doc height = 1 + 3 = 4, max = 4 - 10 = 0 (saturating)
        assert_eq!(state.viewport_offset, 0);
    }

    #[test]
    fn test_file_change_resets_viewport() {
        let mut state = make_state(vec![
            make_file("a.rs", vec![make_hunk_with_lines(20, ReviewStatus::Pending)]),
            make_file("b.rs", vec![make_hunk_with_lines(5, ReviewStatus::Pending)]),
        ]);
        state.viewport_offset = 15;
        state.next_file();
        assert_eq!(state.viewport_offset, 0);
        assert_eq!(state.file_index, 1);
    }

    // --- Edge case tests ---

    #[test]
    fn test_single_hunk_navigation() {
        let mut state = make_state(vec![
            make_file("a.rs", vec![make_hunk(ReviewStatus::Pending)]),
        ]);
        // next_hunk on single hunk should stay at index 0
        state.next_hunk();
        assert_eq!(state.file_index, 0);
        assert_eq!(state.hunk_index, 0);
        // prev_hunk on single hunk should stay at index 0
        state.prev_hunk();
        assert_eq!(state.file_index, 0);
        assert_eq!(state.hunk_index, 0);
    }

    #[test]
    fn test_large_hunk_viewport() {
        let mut state = make_state(vec![
            make_file("a.rs", vec![make_hunk_with_lines(100, ReviewStatus::Pending)]),
        ]);
        state.viewport_height = 20;
        state.ensure_visible();
        // Hunk = 1 header + 100 lines = 101, viewport = 20
        // ensure_visible scrolls to show bottom: 101 - 20 = 81
        assert_eq!(state.viewport_offset, 81);
        // Scroll up
        state.scroll_up(30);
        assert_eq!(state.viewport_offset, 51);
        // Scroll back down to max
        state.scroll_down(100);
        assert_eq!(state.viewport_offset, 81);
    }

    #[test]
    fn test_all_pending_next_pending() {
        let mut state = make_state(vec![
            make_file("a.rs", vec![make_hunk(ReviewStatus::Pending), make_hunk(ReviewStatus::Pending)]),
            make_file("b.rs", vec![make_hunk(ReviewStatus::Pending)]),
        ]);
        // All hunks are Pending; next_pending should move to next hunk
        assert_eq!(state.file_index, 0);
        assert_eq!(state.hunk_index, 0);
        let found = state.next_pending();
        assert!(found);
        assert_eq!(state.file_index, 0);
        assert_eq!(state.hunk_index, 1);
        // Again
        let found = state.next_pending();
        assert!(found);
        assert_eq!(state.file_index, 1);
        assert_eq!(state.hunk_index, 0);
        // Again — should wrap to first hunk
        let found = state.next_pending();
        assert!(found);
        assert_eq!(state.file_index, 0);
        assert_eq!(state.hunk_index, 0);
    }

    #[test]
    fn test_undo_after_toggle_cycle() {
        let mut state = make_state(vec![
            make_file("a.rs", vec![make_hunk(ReviewStatus::Pending)]),
        ]);
        // Toggle 3 times: Pending → Accepted → Rejected → Pending
        state.toggle_current_status();
        assert_eq!(state.current_hunk().unwrap().status, ReviewStatus::Accepted);
        state.toggle_current_status();
        assert_eq!(state.current_hunk().unwrap().status, ReviewStatus::Rejected);
        state.toggle_current_status();
        assert_eq!(state.current_hunk().unwrap().status, ReviewStatus::Pending);
        // Undo 3 times: Pending → Rejected → Accepted → Pending
        state.undo();
        assert_eq!(state.current_hunk().unwrap().status, ReviewStatus::Rejected);
        state.undo();
        assert_eq!(state.current_hunk().unwrap().status, ReviewStatus::Accepted);
        state.undo();
        assert_eq!(state.current_hunk().unwrap().status, ReviewStatus::Pending);
        // Stack should be empty, another undo is a no-op
        state.undo();
        assert_eq!(state.current_hunk().unwrap().status, ReviewStatus::Pending);
    }
}
