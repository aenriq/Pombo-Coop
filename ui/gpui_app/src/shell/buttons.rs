use super::*;
use crate::mock_data::{BadgeTone, DiffViewMode};

impl AppShell {
    pub(super) fn button_palette(&self, kind: ButtonKind, selected: bool) -> (u32, u32) {
        let colors = self.colors();
        match kind {
            ButtonKind::Neutral => {
                if selected {
                    (colors.text_primary, colors.primary)
                } else {
                    (colors.text_muted, colors.text_primary)
                }
            }
            ButtonKind::Primary => {
                if selected {
                    (colors.primary, colors.primary_foreground)
                } else {
                    (colors.text_muted, colors.primary_foreground)
                }
            }
            ButtonKind::Success => {
                if selected {
                    (colors.success, colors.success_foreground)
                } else {
                    (colors.text_muted, colors.success)
                }
            }
            ButtonKind::Destructive => {
                if selected {
                    (colors.destructive, colors.destructive_foreground)
                } else {
                    (0xfca5a5, colors.destructive)
                }
            }
        }
    }

    pub(super) fn button_hover_bg(&self, kind: ButtonKind) -> u32 {
        let colors = self.colors();
        match kind {
            ButtonKind::Neutral => colors.accent,
            ButtonKind::Primary => colors.primary,
            ButtonKind::Success => colors.success,
            ButtonKind::Destructive => colors.destructive,
        }
    }

    fn render_button_frame(&self, size: ButtonSize, variant: ButtonVariant) -> Div {
        let mut button = div()
            .h(px(size.height()))
            .rounded_sm()
            .flex()
            .items_center()
            .cursor_pointer()
            .text_xs();

        button = match variant {
            ButtonVariant::Text => button.px(px(size.horizontal_padding())),
            ButtonVariant::Icon => button.w(px(size.height())).justify_center(),
            ButtonVariant::IconText => button.px(px(size.horizontal_padding())).gap_1(),
        };

        button
    }

    pub(super) fn render_text_button<F>(
        &self,
        label: impl Into<SharedString>,
        kind: ButtonKind,
        size: ButtonSize,
        selected: bool,
        on_click: F,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        F: Fn(&mut Self, &MouseDownEvent, &mut Window, &mut Context<Self>) + 'static,
    {
        let label: SharedString = label.into();
        let (text_color, hover_text_color) = self.button_palette(kind, selected);
        let hover_bg = self.button_hover_bg(kind);
        let text_weight = if selected {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::MEDIUM
        };

        self.render_button_frame(size, ButtonVariant::Text)
            .font_weight(text_weight)
            .text_color(rgb(text_color))
            .hover(move |style| style.bg(rgb(hover_bg)).text_color(rgb(hover_text_color)))
            .on_mouse_move(|_, window, _| window.refresh())
            .on_mouse_down(MouseButton::Left, cx.listener(on_click))
            .child(label)
    }

    pub(super) fn render_icon_button<F>(
        &self,
        icon_path: &'static str,
        kind: ButtonKind,
        size: ButtonSize,
        selected: bool,
        on_click: F,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        F: Fn(&mut Self, &MouseDownEvent, &mut Window, &mut Context<Self>) + 'static,
    {
        let (icon_color, _) = self.button_palette(kind, selected);
        let hover_bg = self.button_hover_bg(kind);
        self.render_button_frame(size, ButtonVariant::Icon)
            .text_color(rgb(icon_color))
            .hover(move |style| style.bg(rgb(hover_bg)))
            .on_mouse_move(|_, window, _| window.refresh())
            .on_mouse_down(MouseButton::Left, cx.listener(on_click))
            .child(
                svg()
                    .w(px(14.0))
                    .h(px(14.0))
                    .path(icon_path)
                    .text_color(rgb(icon_color)),
            )
    }

    pub(super) fn render_icon_text_button<F>(
        &self,
        label: impl Into<SharedString>,
        icon_path: &'static str,
        kind: ButtonKind,
        size: ButtonSize,
        selected: bool,
        on_click: F,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        F: Fn(&mut Self, &MouseDownEvent, &mut Window, &mut Context<Self>) + 'static,
    {
        let label: SharedString = label.into();
        let (text_color, hover_text_color) = self.button_palette(kind, selected);
        let hover_bg = self.button_hover_bg(kind);
        let text_weight = if selected {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::MEDIUM
        };

        self.render_button_frame(size, ButtonVariant::IconText)
            .font_weight(text_weight)
            .text_color(rgb(text_color))
            .hover(move |style| style.bg(rgb(hover_bg)).text_color(rgb(hover_text_color)))
            .on_mouse_move(|_, window, _| window.refresh())
            .on_mouse_down(MouseButton::Left, cx.listener(on_click))
            .child(
                svg()
                    .w(px(14.0))
                    .h(px(14.0))
                    .path(icon_path)
                    .text_color(rgb(text_color)),
            )
            .child(label)
    }

    pub(super) fn render_mode_button(
        &self,
        label: &'static str,
        target_mode: DiffViewMode,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_selected = self.data.diff_mode() == target_mode;

        self.render_text_button(
            label,
            ButtonKind::Primary,
            ButtonSize::Regular,
            is_selected,
            move |this, _, _, cx| this.set_diff_mode(target_mode, cx),
            cx,
        )
    }

    pub(super) fn render_badge(
        &self,
        label: impl Into<SharedString>,
        tone: BadgeTone,
    ) -> impl IntoElement {
        let label = label.into();
        let (bg, border, text) = self.badge_palette(tone);

        div()
            .h(px(20.0))
            .px_2()
            .rounded_full()
            .border_1()
            .border_color(rgb(border))
            .bg(rgb(bg))
            .flex()
            .items_center()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(text))
                    .child(label),
            )
    }

    pub(super) fn badge_palette(&self, tone: BadgeTone) -> (u32, u32, u32) {
        let colors = self.colors();
        match tone {
            BadgeTone::Neutral => (colors.muted, colors.border, colors.muted_foreground),
            BadgeTone::Info => (colors.accent, colors.border, colors.accent_foreground),
            BadgeTone::Success => (colors.success, colors.success, colors.success_foreground),
            BadgeTone::Warning => (colors.warning, colors.warning, colors.warning_foreground),
            BadgeTone::Danger => (
                colors.destructive,
                colors.destructive,
                colors.destructive_foreground,
            ),
        }
    }
}
