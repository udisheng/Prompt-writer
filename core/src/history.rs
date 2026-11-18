use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const HISTORY_LIMIT: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub created_at: String,
    pub idea: String,
    pub template_id: String,
    pub template_title: String,
    pub duration_secs: Option<u32>,
    pub output: String,
    pub negative: Option<String>,
    pub backend: String,
    pub model: String,
    pub over_limit: bool,
    pub warnings: Vec<String>,
}

impl HistoryEntry {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        idea: String,
        template_id: String,
        template_title: String,
        duration_secs: Option<u32>,
        output: String,
        negative: Option<String>,
        backend: String,
        model: String,
        over_limit: bool,
        warnings: Vec<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now().to_rfc3339(),
            idea,
            template_id,
            template_title,
            duration_secs,
            output,
            negative,
            backend,
            model,
            over_limit,
            warnings,
        }
    }
}

pub fn push_history(items: &mut Vec<HistoryEntry>, entry: HistoryEntry) {
    items.insert(0, entry);
    items.truncate(HISTORY_LIMIT);
}

pub fn delete_history(items: &mut Vec<HistoryEntry>, id: &str) -> bool {
    let before = items.len();
    items.retain(|entry| entry.id != id);
    items.len() != before
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(n: usize) -> HistoryEntry {
        HistoryEntry::new(
            format!("idea {n}"),
            "wan22-t2v".into(),
            "Wan".into(),
            None,
            "out".into(),
            None,
            "openrouter".into(),
            "model".into(),
            false,
            vec![],
        )
    }

    #[test]
    fn keeps_newest_fifty() {
        let mut items = Vec::new();
        for n in 0..60 {
            push_history(&mut items, sample(n));
        }
        assert_eq!(items.len(), 50);
        assert!(items[0].idea.contains("59"));
        assert!(items[49].idea.contains("10"));
        let id = items[3].id.clone();
        assert!(delete_history(&mut items, &id));
        assert_eq!(items.len(), 49);
        assert!(!delete_history(&mut items, &id));
    }
}
