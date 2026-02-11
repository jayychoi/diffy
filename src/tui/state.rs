//! 앱 상태

use crate::model::{Diff, FileDiff, Hunk, ReviewStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Help,
    ConfirmQuit,
}

pub struct AppState {
    pub diff: Diff,
    pub file_index: usize,
    pub hunk_index: usize,
    pub scroll_offset: u16,
    pub mode: AppMode,
    pub should_quit: bool,
}

impl AppState {
    pub fn new(diff: Diff) -> Self {
        Self {
            diff,
            file_index: 0,
            hunk_index: 0,
            scroll_offset: 0,
            mode: AppMode::Normal,
            should_quit: false,
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
            }
        }
    }

    pub fn prev_hunk(&mut self) {
        if self.hunk_index > 0 {
            self.hunk_index -= 1;
        } else if self.file_index > 0 {
            self.file_index -= 1;
            if let Some(file) = self.current_file() {
                self.hunk_index = file.hunks.len().saturating_sub(1);
            }
        }
    }

    pub fn next_file(&mut self) {
        if self.file_index + 1 < self.diff.files.len() {
            self.file_index += 1;
            self.hunk_index = 0;
        }
    }

    pub fn prev_file(&mut self) {
        if self.file_index > 0 {
            self.file_index -= 1;
            self.hunk_index = 0;
        }
    }

    pub fn set_current_status(&mut self, status: ReviewStatus) {
        if let Some(hunk) = self.current_hunk_mut() {
            hunk.status = status;
        }
    }

    pub fn toggle_current_status(&mut self) {
        if let Some(hunk) = self.current_hunk_mut() {
            hunk.status = match hunk.status {
                ReviewStatus::Pending => ReviewStatus::Accepted,
                ReviewStatus::Accepted => ReviewStatus::Rejected,
                ReviewStatus::Rejected => ReviewStatus::Pending,
            };
        }
    }

    pub fn set_all_status(&mut self, status: ReviewStatus) {
        for file in &mut self.diff.files {
            for hunk in &mut file.hunks {
                hunk.status = status;
            }
        }
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
