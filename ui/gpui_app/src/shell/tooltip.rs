use super::*;
use gpui::prelude::FluentBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(super) enum TooltipSide {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub(super) struct TooltipSpec {
    pub id_suffix: SharedString,
    pub label: SharedString,
    pub side: TooltipSide,
    pub side_offset: f32,
    pub anchor_width: f32,
    pub anchor_height: f32,
}

impl TooltipSpec {
    pub(super) fn top(
        id_suffix: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self::with_side(id_suffix, label, TooltipSide::Top, 8.0)
    }

    #[allow(dead_code)]
    pub(super) fn bottom(
        id_suffix: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self::with_side(id_suffix, label, TooltipSide::Bottom, 8.0)
    }

    #[allow(dead_code)]
    pub(super) fn left(
        id_suffix: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self::with_side(id_suffix, label, TooltipSide::Left, 8.0)
    }

    #[allow(dead_code)]
    pub(super) fn right(
        id_suffix: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self::with_side(id_suffix, label, TooltipSide::Right, 8.0)
    }

    pub(super) fn with_side(
        id_suffix: impl Into<SharedString>,
        label: impl Into<SharedString>,
        side: TooltipSide,
        side_offset: f32,
    ) -> Self {
        Self {
            id_suffix: id_suffix.into(),
            label: label.into(),
            side,
            side_offset,
            anchor_width: 24.0,
            anchor_height: 24.0,
        }
    }

    #[allow(dead_code)]
    pub(super) fn with_anchor_size(mut self, width: f32, height: f32) -> Self {
        self.anchor_width = width;
        self.anchor_height = height;
        self
    }
}

impl AppShell {
    pub(super) fn render_tooltip(
        &self,
        trigger: impl IntoElement,
        spec: TooltipSpec,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = self.colors();
        let tooltip_id: SharedString = format!("tooltip-{}", spec.id_suffix).into();
        let tooltip_offset = px(spec.side_offset);
        let anchor_width = px(spec.anchor_width);
        let anchor_height = px(spec.anchor_height);
        let tooltip_height = px(22.0);
        let is_visible = self.hovered_tooltip_id.as_ref() == Some(&tooltip_id);
        let tooltip_id_on_enter = tooltip_id.clone();
        let tooltip_id_on_leave = tooltip_id.clone();

        let tooltip_bubble = div()
            .rounded_md()
            .border_1()
            .border_color(rgb(colors.border_strong))
            .bg(rgb(colors.primary))
            .h(tooltip_height)
            .px_2()
            .flex()
            .items_center()
            .shadow(vec![BoxShadow {
                color: hsla(0.0, 0.0, 0.0, 0.22),
                blur_radius: px(8.0),
                spread_radius: px(0.0),
                offset: point(px(0.0), px(2.0)),
            }])
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(colors.primary_foreground))
            .child(spec.label);

        let tooltip_content = div()
            .absolute()
            .opacity(if is_visible { 1.0 } else { 0.0 })
            .when(matches!(spec.side, TooltipSide::Top), |tooltip| {
                tooltip
                    .bottom(anchor_height + tooltip_offset)
                    .left(px(0.0))
                    .right(px(0.0))
                    .h(tooltip_height)
                    .flex()
                    .items_center()
                    .justify_center()
            })
            .when(matches!(spec.side, TooltipSide::Bottom), |tooltip| {
                tooltip
                    .top(anchor_height + tooltip_offset)
                    .left(px(0.0))
                    .right(px(0.0))
                    .h(tooltip_height)
                    .flex()
                    .items_center()
                    .justify_center()
            })
            .when(matches!(spec.side, TooltipSide::Left), |tooltip| {
                tooltip
                    .right(anchor_width + tooltip_offset)
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .flex()
                    .items_center()
                    .justify_center()
            })
            .when(matches!(spec.side, TooltipSide::Right), |tooltip| {
                tooltip
                    .left(anchor_width + tooltip_offset)
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .flex()
                    .items_center()
                    .justify_center()
            })
            .child(tooltip_bubble);

        div()
            .id(tooltip_id.clone())
            .relative()
            .on_hover(cx.listener(move |this, hovered, _, cx| {
                if *hovered {
                    if this.hovered_tooltip_id.as_ref() != Some(&tooltip_id_on_enter) {
                        this.hovered_tooltip_id = Some(tooltip_id_on_enter.clone());
                        cx.notify();
                    }
                } else if this.hovered_tooltip_id.as_ref() == Some(&tooltip_id_on_leave) {
                    this.hovered_tooltip_id = None;
                    cx.notify();
                }
            }))
            .child(trigger)
            .child(deferred(tooltip_content).priority(usize::MAX))
    }
}
