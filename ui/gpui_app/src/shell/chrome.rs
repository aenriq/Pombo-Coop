use super::*;

impl AppShell {
    pub(super) fn pane_border_color(&self, pane: ActivePane) -> Rgba {
        if self.active_pane == pane {
            rgb(self.colors().border_strong)
        } else {
            rgb(self.colors().border)
        }
    }

    pub(super) fn render_active_badge(&self, pane: ActivePane) -> impl IntoElement {
        let is_active = self.active_pane == pane;
        let (bg, border, text, label) = if is_active {
            (
                self.colors().success,
                self.colors().success,
                self.colors().success_foreground,
                "active",
            )
        } else {
            (
                self.colors().card_bg,
                self.colors().border,
                self.colors().text_muted,
                "idle",
            )
        };

        div()
            .h(px(18.0))
            .px_2()
            .rounded_full()
            .border_1()
            .border_color(rgb(border))
            .bg(rgb(bg))
            .flex()
            .items_center()
            .child(div().text_xs().text_color(rgb(text)).child(label))
    }

    pub(super) fn render_pane_header(
        &self,
        pane: ActivePane,
        title: &'static str,
        subtitle: SharedString,
        controls: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .h(px(44.0))
            .px_3()
            .flex()
            .justify_between()
            .items_center()
            .border_b_1()
            .border_color(rgb(self.colors().border))
            .bg(rgb(self.colors().header_bg))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.set_active_pane(pane, cx)),
            )
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
                            .child(title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(self.colors().text_muted))
                            .child(subtitle),
                    )
                    .child(self.render_active_badge(pane)),
            )
            .child(controls)
    }

    pub(super) fn render_top_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = self.colors();
        div()
            .h(px(TOP_BAR_HEIGHT))
            .px_3()
            .border_b_1()
            .border_color(rgb(colors.border))
            .bg(rgb(colors.header_bg))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().w(px(TOP_BAR_TRAFFIC_LIGHT_SPACER)).h(px(1.0)))
                    .child(
                        div()
                            .h(px(22.0))
                            .px_2()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(colors.border))
                            .bg(rgb(colors.card_bg))
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(colors.text_muted))
                                    .child("/kampala-v3"),
                            ),
                    )
                    .child(
                        div()
                            .h(px(22.0))
                            .px_2()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(colors.border))
                            .bg(rgb(colors.card_bg))
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(colors.text_primary))
                                    .child("archive-in-repo-details"),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .h(px(22.0))
                            .px_2()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(colors.success))
                            .bg(rgb(colors.success))
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(colors.success_foreground))
                                    .child("Ready to merge"),
                            ),
                    )
                    .child(self.render_text_button(
                        format!("Neutral {}", self.theme.mode.label()),
                        ButtonKind::Neutral,
                        ButtonSize::Regular,
                        false,
                        |this, _, _, cx| this.toggle_theme_mode(cx),
                        cx,
                    )),
            )
    }

    pub(super) fn render_left_splitter(&self) -> impl IntoElement {
        let hover_color = self.colors().splitter_hover;
        div()
            .id("left-pane-splitter-track")
            .relative()
            .h_full()
            .w(px(SPLITTER_TRACK_WIDTH))
            .flex_shrink_0()
            .bg(rgb(self.colors().border))
            .child(
                div()
                    .id("left-pane-splitter-handle")
                    .absolute()
                    .left(px(-(SPLITTER_HIT_WIDTH - SPLITTER_TRACK_WIDTH) / 2.0))
                    .h_full()
                    .w(px(SPLITTER_HIT_WIDTH))
                    .cursor_col_resize()
                    .block_mouse_except_scroll()
                    .hover(move |style| style.bg(rgb(hover_color)))
                    .on_mouse_move(|_, window, _| window.refresh())
                    .on_drag(DraggedLeftPaneHandle, |_, _, _, cx| cx.new(|_| gpui::Empty)),
            )
    }

    pub(super) fn render_right_splitter(&self) -> impl IntoElement {
        let hover_color = self.colors().splitter_hover;
        div()
            .id("right-pane-splitter-track")
            .relative()
            .h_full()
            .w(px(SPLITTER_TRACK_WIDTH))
            .flex_shrink_0()
            .bg(rgb(self.colors().border))
            .child(
                div()
                    .id("right-pane-splitter-handle")
                    .absolute()
                    .left(px(-(SPLITTER_HIT_WIDTH - SPLITTER_TRACK_WIDTH) / 2.0))
                    .h_full()
                    .w(px(SPLITTER_HIT_WIDTH))
                    .cursor_col_resize()
                    .block_mouse_except_scroll()
                    .hover(move |style| style.bg(rgb(hover_color)))
                    .on_mouse_move(|_, window, _| window.refresh())
                    .on_drag(DraggedRightPaneHandle, |_, _, _, cx| {
                        cx.new(|_| gpui::Empty)
                    }),
            )
    }

    pub(super) fn render_shortcut_legend(&self) -> impl IntoElement {
        div()
            .id("shortcut-legend")
            .h(px(38.0))
            .px_3()
            .flex()
            .items_center()
            .gap_2()
            .border_t_1()
            .border_color(rgb(self.colors().border))
            .bg(rgb(self.colors().header_bg))
            .overflow_x_scroll()
            .child(self.render_shortcut_chip("Alt+1/2/3", "Focus pane"))
            .child(self.render_shortcut_chip("Ctrl+Tab", "Next pane"))
            .child(self.render_shortcut_chip("Ctrl+Shift+Tab", "Prev pane"))
            .child(self.render_shortcut_chip("Up/Down or J/K", "File nav"))
            .child(self.render_shortcut_chip("D", "Toggle unified/split"))
            .child(self.render_shortcut_chip("Cmd/Ctrl+Shift+L", "Toggle theme"))
            .child(self.render_shortcut_chip("S", "Stage / unstage"))
            .child(self.render_shortcut_chip("R", "Revert selected"))
            .child(self.render_shortcut_chip("Alt+←/→", "Resize side pane"))
    }

    pub(super) fn render_shortcut_chip(
        &self,
        keystroke: &'static str,
        description: &'static str,
    ) -> impl IntoElement {
        div()
            .h(px(24.0))
            .px_2()
            .rounded_sm()
            .border_1()
            .border_color(rgb(self.colors().border))
            .bg(rgb(self.colors().card_bg))
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(self.colors().text_primary))
                    .child(keystroke),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(self.colors().text_muted))
                    .child(description),
            )
    }
}
