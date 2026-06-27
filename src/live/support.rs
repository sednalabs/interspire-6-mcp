use crate::response::ListSummary;

pub(super) fn cap_usize(value: usize, max: usize) -> usize {
    value.clamp(1, max)
}

pub(super) fn apply_list_result_cap(
    lists: &mut Vec<ListSummary>,
    max_lists: usize,
    label: &str,
    warnings: &mut Vec<String>,
    notes: &mut Vec<String>,
) {
    let original_count = lists.len();
    if original_count <= max_lists {
        return;
    }

    lists.truncate(max_lists);
    warnings.push(format!(
        "XML list readback returned {original_count} lists; {label} applied max_lists cap {max_lists}"
    ));
    notes.push(format!(
        "{label} XML results truncated from {original_count} lists to applied cap {max_lists}"
    ));
}

pub(super) fn filter_requested_source_lists(
    lists: Vec<ListSummary>,
    requested_source_list_ids: &[u64],
) -> Vec<ListSummary> {
    let mut selected = Vec::new();
    let mut remaining = lists;
    let mut seen = Vec::new();

    for list_id in requested_source_list_ids
        .iter()
        .copied()
        .filter(|list_id| *list_id > 0)
    {
        if seen.contains(&list_id) {
            continue;
        }
        seen.push(list_id);

        if let Some(index) = remaining
            .iter()
            .position(|candidate| candidate.list_id == list_id)
        {
            selected.push(remaining.remove(index));
        }
    }

    selected
}

pub(super) fn join_ids_for_warning(values: &[u64]) -> String {
    values
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::{apply_list_result_cap, filter_requested_source_lists};
    use crate::response::{
        Evidence, ListSummary, WarmupAudienceReadinessReport, WarmupAudienceReadinessRequest,
    };

    fn list_summary(list_id: u64) -> ListSummary {
        ListSummary {
            list_id,
            name: format!("List {list_id}"),
            subscribed_count: Some(list_id * 10),
            unsubscribed_count: Some(list_id),
            autoresponder_count: Some(0),
            owner_name: None,
            owner_email_redacted: None,
            reply_to_email_redacted: None,
            bounce_email_redacted: None,
            source: "xml".to_string(),
        }
    }

    #[test]
    fn owner_cap_records_warning_and_evidence() {
        let mut lists = vec![list_summary(1), list_summary(2), list_summary(3)];
        let mut warnings = Vec::new();
        let mut notes = Vec::new();

        apply_list_result_cap(
            &mut lists,
            2,
            "list owner readback",
            &mut warnings,
            &mut notes,
        );

        assert_eq!(
            lists.iter().map(|list| list.list_id).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("returned 3 lists")
                && warning.contains("applied max_lists cap 2")));
        assert!(notes
            .iter()
            .any(|note| note.contains("truncated from 3 lists to applied cap 2")));
    }

    #[test]
    fn list_summary_cap_uses_explicit_label_in_warning_and_evidence() {
        let mut lists = vec![list_summary(1), list_summary(2), list_summary(3)];
        let mut warnings = Vec::new();
        let mut notes = Vec::new();

        apply_list_result_cap(&mut lists, 1, "list summary", &mut warnings, &mut notes);

        assert_eq!(
            lists.iter().map(|list| list.list_id).collect::<Vec<_>>(),
            vec![1]
        );
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("list summary applied max_lists cap 1")));
        assert!(notes
            .iter()
            .any(|note| note.contains("list summary XML results truncated from 3 lists")));
    }

    #[test]
    fn warmup_filter_keeps_only_requested_lists_in_request_order() {
        let lists = vec![list_summary(1), list_summary(2), list_summary(3)];

        let filtered = filter_requested_source_lists(lists, &[3, 9, 1, 3, 0]);

        assert_eq!(
            filtered.iter().map(|list| list.list_id).collect::<Vec<_>>(),
            vec![3, 1]
        );
    }

    #[test]
    fn warmup_filter_still_allows_missing_list_detection() {
        let request = WarmupAudienceReadinessRequest {
            source_list_ids: vec![72, 111, 114],
            priority_list_ids: Vec::new(),
            tranche_sizes: vec![10],
            include_html_enrichment: true,
        };
        let filtered =
            filter_requested_source_lists(vec![list_summary(111)], &request.source_list_ids);

        let report = WarmupAudienceReadinessReport::from_lists(
            &request,
            filtered,
            Vec::new(),
            Evidence {
                source: "test".to_string(),
                notes: Vec::new(),
            },
        );

        assert_eq!(report.missing_list_ids, vec![72, 114]);
        assert_eq!(report.gross_subscribed_count, 1110);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning == "missing specified source list ids: 72, 114"));
    }
}
