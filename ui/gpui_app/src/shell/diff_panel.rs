use super::*;
use crate::mock_data::{
    DiffHunk, DiffLineKind, DiffRows, DiffViewMode, SplitDiffCell, SplitDiffRow, UnifiedDiffLine,
};

impl AppShell {
    pub(super) fn render_diff_viewer_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_label = self
            .data
            .selected_file()
            .map(|file| file.path.clone())
            .unwrap_or_else(|| "No file selected".into());

        let controls = div()
            .flex()
            .items_center()
            .gap_1()
            .child(self.render_mode_button("Unified", DiffViewMode::Unified, cx))
            .child(self.render_mode_button("Split", DiffViewMode::Split, cx));

        let mut body = div()
            .id("diff-pane-body")
            .flex_1()
            .overflow_y_scroll()
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(self.colors().border))
                    .bg(rgb(self.colors().card_bg))
                    .px_3()
                    .py_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(self.colors().text_muted))
                            .child(format!("Selected: {}", selected_label)),
                    ),
            );

        if let Some(diff) = self.data.selected_diff() {
            let mode = self.data.diff_mode();
            body = body.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(self.colors().border))
                    .bg(rgb(self.colors().card_bg))
                    .px_3()
                    .py_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(self.colors().text_primary))
                                    .child(diff.file_path.clone()),
                            )
                            .child(self.render_badge(diff.kind.code(), diff.kind.tone())),
                    ),
            );

            body = body.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(self.colors().border))
                    .bg(rgb(self.colors().panel_bg))
                    .p_2()
                    .child(
                        div()
                            .text_xs()
                            .font_family(".ZedMono")
                            .text_color(rgb(self.colors().text_muted))
                            .child(diff.text_for_mode(mode).clone()),
                    ),
            );

            for hunk in &diff.hunks {
                body = body.child(self.render_diff_hunk(hunk, mode));
            }
        } else {
            body = body.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(self.colors().border))
                    .bg(rgb(self.colors().card_bg))
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(self.colors().text_muted))
                            .child("Diff scaffold will appear when a file is selected."),
                    ),
            );
        }

        div()
            .id("diff-pane")
            .h_full()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(460.0))
            .overflow_hidden()
            .bg(rgb(self.colors().panel_bg))
            .border_1()
            .border_color(self.pane_border_color(ActivePane::Middle))
            .child(
                self.render_pane_header(
                    ActivePane::Middle,
                    "Diff Viewer",
                    if self.data.diff_mode().eq(&DiffViewMode::Unified) {
                        "unified"
                    } else {
                        "split"
                    }
                    .into(),
                    controls,
                    cx,
                ),
            )
            .child(
                div()
                    .id("middle-pane-content")
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .flex_col()
                    .child(body)
                    .child(self.render_chat_panel()),
            )
    }

    pub(super) fn render_chat_panel(&self) -> impl IntoElement {
        div()
            .id("middle-chat-panel")
            .h(px(220.0))
            .flex_none()
            .border_t_1()
            .border_color(rgb(self.colors().border))
            .bg(rgb(self.colors().header_bg))
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .id("middle-chat-thread")
                    .flex_1()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(self.render_chat_message(
                        "Assistant",
                        "Perfect! Added missing imports and verified the dialog compiles cleanly.",
                        false,
                    ))
                    .child(self.render_chat_message(
                        "Reviewer",
                        "Could we reuse command-style keyboard navigation in this dialog?",
                        true,
                    ))
                    .child(self.render_chat_message(
                        "Assistant",
                        "Yes. I can wire list navigation with up/down + enter in the same pass.",
                        false,
                    )),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(self.colors().border))
                    .bg(rgb(self.colors().card_bg))
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .h(px(64.0))
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(self.colors().border))
                            .bg(rgb(self.colors().panel_bg))
                            .px_2()
                            .py_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(self.colors().text_muted))
                                    .child("Message agent about this diff..."),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(self.render_shortcut_chip("Sonnet 4.5", "model"))
                                    .child(self.render_shortcut_chip("Link issue", "action")),
                            )
                            .child(
                                div()
                                    .h(px(24.0))
                                    .px_3()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(rgb(self.colors().success))
                                    .bg(rgb(self.colors().success))
                                    .flex()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(self.colors().success_foreground))
                                            .child("Send"),
                                    ),
                            ),
                    ),
            )
    }

    pub(super) fn render_chat_message(
        &self,
        author: &'static str,
        text: &'static str,
        outgoing: bool,
    ) -> impl IntoElement {
        let (bubble_bg, bubble_border, author_color, text_color) = if outgoing {
            (0x2f2a22, 0x5b4d3f, 0xf5ddb0, self.colors().text_primary)
        } else {
            (
                self.colors().card_bg,
                self.colors().border,
                self.colors().success_foreground,
                self.colors().text_primary,
            )
        };

        let mut row = div().flex();
        row = if outgoing {
            row.justify_end()
        } else {
            row.justify_start()
        };

        row.child(
            div()
                .max_w(px(680.0))
                .rounded_md()
                .border_1()
                .border_color(rgb(bubble_border))
                .bg(rgb(bubble_bg))
                .px_3()
                .py_2()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(author_color))
                        .child(author),
                )
                .child(div().text_sm().text_color(rgb(text_color)).child(text)),
        )
    }

    pub(super) fn render_diff_hunk(&self, hunk: &DiffHunk, mode: DiffViewMode) -> impl IntoElement {
        let mut rows = div().flex().flex_col().gap_1();

        match hunk.rows(mode) {
            DiffRows::Unified(lines) => {
                for line in lines {
                    rows = rows.child(self.render_unified_line(line));
                }
            }
            DiffRows::Split(split_rows) => {
                for row in split_rows {
                    rows = rows.child(self.render_split_row(row));
                }
            }
        }

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(self.colors().border))
            .bg(rgb(self.colors().card_bg))
            .p_2()
            .child(
                div()
                    .mb_1()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(self.colors().text_muted))
                    .font_family(".ZedMono")
                    .child(hunk.header.clone()),
            )
            .child(rows)
            .child(
                div()
                    .mt_1()
                    .rounded_sm()
                    .bg(rgb(self.colors().panel_bg))
                    .px_2()
                    .py_1()
                    .child(
                        div()
                            .text_xs()
                            .font_family(".ZedMono")
                            .text_color(rgb(self.colors().text_muted))
                            .child(hunk.text_for_mode(mode).clone()),
                    ),
            )
    }

    pub(super) fn render_unified_line(&self, line: &UnifiedDiffLine) -> impl IntoElement {
        let (bg, text) = self.line_palette(line.kind);

        div()
            .font_family(".ZedMono")
            .text_xs()
            .flex()
            .items_center()
            .gap_2()
            .rounded_sm()
            .bg(rgb(bg))
            .px_2()
            .py_1()
            .child(
                div()
                    .w(px(38.0))
                    .text_color(rgb(self.colors().text_muted))
                    .child(Self::line_number_text(line.old_line_number)),
            )
            .child(
                div()
                    .w(px(38.0))
                    .text_color(rgb(self.colors().text_muted))
                    .child(Self::line_number_text(line.new_line_number)),
            )
            .child(
                div()
                    .w(px(12.0))
                    .text_color(rgb(text))
                    .child(line.kind.prefix().to_string()),
            )
            .child(
                div()
                    .flex_1()
                    .text_color(rgb(text))
                    .child(line.text.clone()),
            )
    }

    pub(super) fn render_split_row(&self, row: &SplitDiffRow) -> impl IntoElement {
        div()
            .flex()
            .gap_2()
            .child(self.render_split_cell(row.left.as_ref()))
            .child(self.render_split_cell(row.right.as_ref()))
    }

    pub(super) fn render_split_cell(&self, cell: Option<&SplitDiffCell>) -> impl IntoElement {
        let mut container = div()
            .flex_1()
            .min_h(px(24.0))
            .rounded_sm()
            .border_1()
            .border_color(rgb(self.colors().border))
            .px_2()
            .py_1()
            .font_family(".ZedMono")
            .text_xs();

        if let Some(cell) = cell {
            let (bg, text) = self.line_palette(cell.kind);
            container = container.bg(rgb(bg)).child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w(px(34.0))
                            .text_color(rgb(self.colors().text_muted))
                            .child(Self::line_number_text(cell.line_number)),
                    )
                    .child(
                        div()
                            .w(px(12.0))
                            .text_color(rgb(text))
                            .child(cell.kind.prefix().to_string()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(rgb(text))
                            .child(cell.text.clone()),
                    ),
            );
        } else {
            container = container.bg(rgb(self.colors().panel_bg));
        }

        container
    }

    pub(super) fn line_palette(&self, kind: DiffLineKind) -> (u32, u32) {
        match kind {
            DiffLineKind::Context => (self.colors().panel_bg, self.colors().text_primary),
            DiffLineKind::Added => (0x193424, 0xa7f0c4),
            DiffLineKind::Removed => (0x4a2424, 0xf2b5b5),
        }
    }

    pub(super) fn line_number_text(line_number: Option<u32>) -> SharedString {
        match line_number {
            Some(line_number) => format!("{line_number:>4}").into(),
            None => "    ".into(),
        }
    }
}
