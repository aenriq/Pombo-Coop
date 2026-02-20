use super::*;
use crate::mock_data::{ChangedFile, FileChangeKind, FileStageGroup};

impl AppShell {
    pub(super) fn single_line_file_button_style(
        &self,
        file: &ChangedFile,
    ) -> (&'static str, u32, u32) {
        let colors = self.colors();
        if file.kind == FileChangeKind::Deleted {
            return (ICON_SQUARE_MINUS, colors.destructive, 0xfecaca);
        }

        if file.kind == FileChangeKind::Added
            || file.stage_group == FileStageGroup::Untracked
            || file.stage_group == FileStageGroup::Staged
        {
            return (ICON_SQUARE_PLUS, colors.success, 0xbbf7d0);
        }

        if file.kind == FileChangeKind::Renamed {
            return (ICON_SQUARE_DOT, colors.warning, 0xfef3c7);
        }

        if file.kind == FileChangeKind::Conflict || file.stage_group == FileStageGroup::Conflict {
            return (ICON_SQUARE_DOT, colors.warning, 0xfde68a);
        }

        (ICON_SQUARE_DOT, colors.warning, 0xfde68a)
    }

    pub(super) fn render_single_line_file_button(&self, file: &ChangedFile) -> impl IntoElement {
        let (icon_path, color, hover_color) = self.single_line_file_button_style(file);

        div()
            .w(px(14.0))
            .h(px(14.0))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_move(|_, window, _| window.refresh())
            .child(
                svg()
                    .w(px(14.0))
                    .h(px(14.0))
                    .path(icon_path)
                    .text_color(rgb(color))
                    .hover(move |style| style.text_color(rgb(hover_color)))
                    .on_mouse_move(|_, window, _| window.refresh()),
            )
    }

    pub(super) fn render_changed_file_diff_summary(&self, file: &ChangedFile) -> impl IntoElement {
        let colors = self.colors();
        let mut summary = div()
            .flex()
            .items_center()
            .gap_1()
            .font_family(".ZedMono")
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD);

        if file.additions > 0 {
            summary = summary.child(
                div()
                    .text_color(rgb(colors.success))
                    .child(format!("+{}", file.additions)),
            );
        }

        if file.deletions > 0 || file.additions == 0 {
            summary = summary.child(
                div()
                    .text_color(rgb(colors.destructive))
                    .child(format!("-{}", file.deletions)),
            );
        }

        summary
    }

    pub(super) fn render_changed_files_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let subtitle: SharedString = format!("{:.0}px", self.right_pane_width).into();
        let controls = div();
        let lane_files = self.data.changed_files_for_lane(self.selected_lane_id);
        let lane_file_count = lane_files.len();

        let mut files = div().id("changed-files-list").flex().flex_col().gap_0p5();
        for file in lane_files {
            files = files.child(self.render_changed_file_row(file, cx));
        }

        let selected_stage = self
            .data
            .selected_file()
            .filter(|file| {
                self.selected_lane_id
                    .map(|lane_id| file.owner_lane_id == Some(lane_id))
                    .unwrap_or(true)
            })
            .map(|file| file.stage_group.label().to_string())
            .unwrap_or_else(|| "none".to_string());

        div()
            .id("changed-files-pane")
            .h_full()
            .w(px(self.right_pane_width))
            .flex_none()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(self.colors().panel_bg))
            .border_1()
            .border_color(self.pane_border_color(ActivePane::Right))
            .child(self.render_pane_header(
                ActivePane::Right,
                "Changed Files",
                subtitle,
                controls,
                cx,
            ))
            .child(
                div()
                    .flex_1()
                    .id("changed-files-scroll")
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
                                    .flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(self.colors().text_muted))
                                            .child(format!(
                                                "{} files • selected stage {}",
                                                lane_file_count,
                                                selected_stage
                                            )),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .child(self.render_text_button(
                                                "Stage (S)",
                                                ButtonKind::Success,
                                                ButtonSize::Compact,
                                                false,
                                                |this, _, _, cx| {
                                                    this.stage_or_unstage_selected_file(cx);
                                                },
                                                cx,
                                            ))
                                            .child(self.render_text_button(
                                                "Revert (R)",
                                                ButtonKind::Destructive,
                                                ButtonSize::Compact,
                                                false,
                                                |this, _, _, cx| {
                                                    this.revert_selected_file(cx);
                                                },
                                                cx,
                                            )),
                                    ),
                            ),
                    )
                    .child(files),
            )
    }

    pub(super) fn render_changed_file_row(
        &self,
        file: &ChangedFile,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = self.colors();
        let is_selected = self
            .data
            .selected_file_id()
            .map(|selected_id| selected_id == file.id)
            .unwrap_or(false);

        let file_id = file.id;
        let selected_bg = if is_selected {
            colors.secondary
        } else {
            colors.panel_bg
        };
        let hover_bg = if is_selected {
            colors.secondary
        } else {
            colors.accent
        };
        let path_color = if is_selected {
            colors.secondary_foreground
        } else {
            colors.text_primary
        };

        div()
            .h(px(26.0))
            .rounded_sm()
            .bg(rgb(selected_bg))
            .px_2()
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .hover(move |style| style.bg(rgb(hover_bg)))
            .on_mouse_move(|_, window, _| window.refresh())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    if this.data.select_file_by_id(file_id) {
                        this.active_pane = ActivePane::Right;
                        this.status_text = "Selected file from changed files list".into();
                        cx.notify();
                    }
                }),
            )
            .child(
                div().flex_1().min_w(px(0.0)).overflow_hidden().child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(path_color))
                        .child(file.path.clone()),
                ),
            )
            .child(self.render_changed_file_diff_summary(file))
            .child(self.render_single_line_file_button(file))
    }
}
