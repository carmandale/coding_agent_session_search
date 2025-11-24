use coding_agent_search::ui::tui::footer_legend;

#[test]
fn footer_mentions_editor_and_clear_keys() {
    let long = footer_legend(false);
    assert!(long.contains("Enter/F8 open"));
    assert!(long.contains("Ctrl+Del clear"));
    assert!(long.contains("Esc/F10 quit"));
}
