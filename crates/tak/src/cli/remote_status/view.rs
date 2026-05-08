use super::{RemoteRecord, RemoteStatusResult};

#[derive(Clone, Debug)]
pub(super) struct RemoteStatusView {
    rows: Vec<RemoteStatusRow>,
    pub(super) poll_index: usize,
    pub(super) watch: bool,
    pub(super) tick: usize,
}

#[derive(Clone, Debug)]
pub(super) enum RemoteStatusRow {
    Checking { remote: RemoteRecord },
    Complete(Box<RemoteStatusResult>),
}

impl RemoteStatusView {
    pub(super) fn checking(remotes: &[RemoteRecord], poll_index: usize, watch: bool) -> Self {
        let mut rows = remotes
            .iter()
            .cloned()
            .map(|remote| RemoteStatusRow::Checking { remote })
            .collect::<Vec<_>>();
        rows.sort_unstable_by(|left, right| left.node_id().cmp(right.node_id()));
        Self {
            rows,
            poll_index,
            watch,
            tick: 0,
        }
    }

    pub(super) fn advance(&mut self) {
        self.tick = self.tick.saturating_add(1);
    }

    pub(super) fn mark_complete(&mut self, result: RemoteStatusResult) {
        let node_id = result.remote.node_id.as_str();
        if let Some(row) = self.rows.iter_mut().find(|row| row.node_id() == node_id) {
            *row = RemoteStatusRow::Complete(Box::new(result));
        } else {
            self.rows.push(RemoteStatusRow::Complete(Box::new(result)));
        }
        self.rows
            .sort_unstable_by(|left, right| left.node_id().cmp(right.node_id()));
    }

    pub(super) fn rows(&self) -> &[RemoteStatusRow] {
        &self.rows
    }

    #[cfg(test)]
    pub(super) fn node_ids(&self) -> Vec<&str> {
        self.rows.iter().map(RemoteStatusRow::node_id).collect()
    }

    pub(super) fn total_count(&self) -> usize {
        self.rows.len()
    }

    pub(super) fn checking_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| matches!(row, RemoteStatusRow::Checking { .. }))
            .count()
    }

    pub(super) fn completed_count(&self) -> usize {
        self.total_count().saturating_sub(self.checking_count())
    }

    pub(super) fn completed_results(&self) -> Vec<RemoteStatusResult> {
        self.rows
            .iter()
            .filter_map(|row| match row {
                RemoteStatusRow::Checking { .. } => None,
                RemoteStatusRow::Complete(result) => Some((**result).clone()),
            })
            .collect()
    }

    #[cfg(test)]
    pub(super) fn has_errors(&self) -> bool {
        self.rows.iter().any(|row| match row {
            RemoteStatusRow::Checking { .. } => false,
            RemoteStatusRow::Complete(result) => result.error.is_some(),
        })
    }
}

impl RemoteStatusRow {
    pub(super) fn node_id(&self) -> &str {
        self.remote().node_id.as_str()
    }

    pub(super) fn remote(&self) -> &RemoteRecord {
        match self {
            RemoteStatusRow::Checking { remote } => remote,
            RemoteStatusRow::Complete(result) => &result.remote,
        }
    }
}
