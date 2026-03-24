// 📂 File: tests/test_log.rs
#[cfg(test)]
mod tests {
    use core_api::recovery::{UnifiedEntry, compact_unified_log};


    #[test]
    fn test_compaction_removes_redundant_scheduler() {
        let mut entries = vec![
            UnifiedEntry::Scheduler("cycle".into()),
            UnifiedEntry::Scheduler("cycle".into()),
            UnifiedEntry::Encrypt(vec![1]),
        ];
        compact_unified_log(&mut entries);
        assert_eq!(entries.len(), 2);
    }
}
