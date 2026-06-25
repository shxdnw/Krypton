use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render a full-screen help panel listing every keybinding grouped by view.
pub fn render(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title("Help — Keybindings")
        .borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let section_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let key_style = Style::default().fg(Color::Yellow);
    let desc_style = Style::default().fg(Color::White);
    let dim_style = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::from(Span::styled("─ Global", section_style)),
        line("q", "Quit", key_style, desc_style),
        line("Esc", "Back / cancel", key_style, desc_style),
        line("Ctrl+L", "Lock vault from any view", key_style, desc_style),
        line("?", "Show this help", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("─ Entry List", section_style)),
        line("j / ↓", "Move down (vim: j when enabled)", key_style, desc_style),
        line("k / ↑", "Move up (vim: k when enabled)", key_style, desc_style),
        line("g / G", "Page up / down (vim)", key_style, desc_style),
        line("Enter", "Open selected entry", key_style, desc_style),
        line("n", "New entry", key_style, desc_style),
        line("e", "Edit selected entry", key_style, desc_style),
        line("d", "Delete selected entry", key_style, desc_style),
        line("y", "Copy password", key_style, desc_style),
        line("u", "Copy username", key_style, desc_style),
        line("c", "Copy URL", key_style, desc_style),
        line("/", "Search entries", key_style, desc_style),
        line("s", "Open settings", key_style, desc_style),
        line("L", "Lock vault", key_style, desc_style),
        line("Ctrl+E", "Export entries", key_style, desc_style),
        line("Ctrl+I", "Import entries", key_style, desc_style),
        line("Ctrl+D", "Duplicate entry", key_style, desc_style),
        line("?", "Help", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("─ Entry Detail", section_style)),
        line("p", "Show / hide password", key_style, desc_style),
        line("e", "Edit entry", key_style, desc_style),
        line("d", "Delete entry", key_style, desc_style),
        line("y", "Copy password", key_style, desc_style),
        line("u", "Copy username", key_style, desc_style),
        line("c", "Copy URL", key_style, desc_style),
        line("Esc / q", "Back to list", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("─ Entry Editor", section_style)),
        line("Tab / ↓ / →", "Next field", key_style, desc_style),
        line("Shift+Tab / ↑ / ←", "Previous field", key_style, desc_style),
        line("Ctrl+G", "Generate password (on pwd field)", key_style, desc_style),
        line("Ctrl+S", "Save entry", key_style, desc_style),
        line("Esc", "Discard and go back", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("─ Search", section_style)),
        line("Type to search", "Live-filter results", key_style, desc_style),
        line("Enter", "Open selected result", key_style, desc_style),
        line("y", "Copy password", key_style, desc_style),
        line("u", "Copy username", key_style, desc_style),
        line("e", "Edit result", key_style, desc_style),
        line("d", "Delete result", key_style, desc_style),
        line("Esc", "Back to list", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("─ Settings", section_style)),
        line("j / k or ↓ / ↑", "Navigate rows", key_style, desc_style),
        line("Enter / Space", "Toggle / edit row", key_style, desc_style),
        line("Ctrl+S", "Save settings", key_style, desc_style),
        line("Esc", "Back to list", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("─ Lock / First-Run", section_style)),
        line("Type to enter", "Enter master password", key_style, desc_style),
        line("Enter", "Submit", key_style, desc_style),
        line("Ctrl+H / V", "Show / hide password", key_style, desc_style),
        line("Ctrl+R", "Reset vault (destructive)", key_style, desc_style),
        line("Esc", "Quit", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled("Press Esc, q, or ? to close help", dim_style)),
    ];

    let content = Paragraph::new(lines).scroll((0, 0));
    f.render_widget(content, inner);
}

fn line<'a>(key: &'a str, desc: &'a str, key_style: Style, desc_style: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {key:<22}"), key_style),
        Span::styled(desc, desc_style),
    ])
}
