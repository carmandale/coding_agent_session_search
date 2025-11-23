use coding_agent_search::ui::tui::footer_legend;

#[test]
fn footer_legend_toggles_help() {
    assert!(footer_legend(false).contains("?/hide help"));
    assert!(footer_legend(true).contains("q/esc quit"));
}
