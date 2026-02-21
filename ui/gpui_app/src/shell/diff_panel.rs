use super::*;
use super::textarea::ExpandableTextAreaSpec;
use crate::mock_data::{
    DiffHunk, DiffLineKind, DiffRows, DiffViewMode, SplitDiffCell, SplitDiffRow, UnifiedDiffLine,
};

impl AppShell {
    pub(super) fn render_diff_viewer_pane(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(self.render_chat_panel(cx)),
            )
    }

    pub(super) fn render_chat_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut thread = div()
            .id("middle-chat-thread")
            .flex_1()
            .overflow_y_scroll()
            .track_scroll(&self.chat_thread_scroll)
            .flex()
            .flex_col()
            .gap_2();

        for message in &self.chat_messages {
            thread = thread.child(self.render_chat_message(
                message.author.clone(),
                message.text.clone(),
                message.outgoing,
            ));
        }

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
            .child(thread)
            .child(self.render_expandable_text_area(
                ExpandableTextAreaSpec::new(
                    "middle-chat",
                    "Ask for follow-up changes",
                    2,
                    8,
                ),
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_0p5()
                            .child(self.render_icon_button(
                                ICON_SQUARE_PLUS,
                                ButtonKind::Neutral,
                                ButtonSize::Regular,
                                false,
                                |this, _, _, cx| {
                                    this.active_pane = ActivePane::Middle;
                                    this.status_text = "Attach context action coming soon".into();
                                    cx.notify();
                                },
                                cx,
                            ))
                            .child(self.render_icon_button(
                                ICON_SQUARE_DOT,
                                ButtonKind::Neutral,
                                ButtonSize::Regular,
                                false,
                                |this, _, _, cx| {
                                    this.active_pane = ActivePane::Middle;
                                    this.status_text = "Model picker action coming soon".into();
                                    cx.notify();
                                },
                                cx,
                            ))
                            .child(self.render_icon_button(
                                ICON_CHEVRON_DOWN,
                                ButtonKind::Neutral,
                                ButtonSize::Regular,
                                false,
                                |this, _, _, cx| {
                                    this.active_pane = ActivePane::Middle;
                                    this.status_text = "Model menu action coming soon".into();
                                    cx.notify();
                                },
                                cx,
                            ))
                            .child(self.render_icon_button(
                                ICON_GIT_BRANCH,
                                ButtonKind::Neutral,
                                ButtonSize::Regular,
                                false,
                                |this, _, _, cx| {
                                    this.active_pane = ActivePane::Middle;
                                    this.status_text = "Reasoning mode action coming soon".into();
                                    cx.notify();
                                },
                                cx,
                            ))
                            .child(self.render_icon_button(
                                ICON_CHEVRON_DOWN,
                                ButtonKind::Neutral,
                                ButtonSize::Regular,
                                false,
                                |this, _, _, cx| {
                                    this.active_pane = ActivePane::Middle;
                                    this.status_text = "Reasoning menu action coming soon".into();
                                    cx.notify();
                                },
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(self.render_icon_button(
                                ICON_MIC,
                                ButtonKind::Neutral,
                                ButtonSize::Regular,
                                false,
                                |this, _, _, cx| {
                                    this.active_pane = ActivePane::Middle;
                                    this.status_text = "Voice input action coming soon".into();
                                    cx.notify();
                                },
                                cx,
                            ))
                            .child(self.render_circular_icon_button(
                                ICON_ARROW_UP,
                                ButtonKind::Primary,
                                34.0,
                                |this, _, _, cx| this.submit_composer_message(cx),
                                cx,
                            )),
                    ),
                cx,
            ))
    }

    pub(super) fn render_chat_message(
        &self,
        author: SharedString,
        text: SharedString,
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
