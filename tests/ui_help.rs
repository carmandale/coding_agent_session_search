use coding_agent_search::ui::tui::footer_legend;

#[test]
fn help_legend_has_hotkeys() {
    let short = footer_legend(false);
    assert!(short.contains("a agent"));
    let long = footer_legend(true);
    assert!(long.contains("q/esc"));
}
