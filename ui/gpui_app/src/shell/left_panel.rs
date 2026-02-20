use super::*;
use crate::mock_data::{AgentLane, AgentLaneStatus};
use std::time::Duration;

impl AppShell {
    pub(super) fn lane_repo_groups(&self) -> Vec<(SharedString, Vec<AgentLane>)> {
        let mut groups: Vec<(SharedString, Vec<AgentLane>)> = Vec::new();

        for lane in self.data.agent_lanes() {
            if let Some((_, grouped_lanes)) = groups
                .iter_mut()
                .find(|(repo_name, _)| repo_name == &lane.repo_name)
            {
                grouped_lanes.push(lane.clone());
            } else {
                groups.push((lane.repo_name.clone(), vec![lane.clone()]));
            }
        }

        groups
    }

    pub(super) fn is_repo_group_collapsed(&self, repo_name: &str) -> bool {
        self.collapsed_repo_groups.contains(repo_name)
    }

    pub(super) fn repo_group_animation_version(&self, repo_name: &str) -> u64 {
        self.repo_group_animation_versions
            .get(repo_name)
            .copied()
            .unwrap_or(0)
    }

    pub(super) fn repo_group_body_height(&self, lane_count: usize) -> f32 {
        if lane_count == 0 {
            return 0.0;
        }

        let rows_height = lane_count as f32 * LANE_ROW_ESTIMATED_HEIGHT;
        let gaps_height = (lane_count.saturating_sub(1) as f32) * LANE_ROW_GAP;
        let per_row_margin = lane_count as f32 * LANE_ROW_HEIGHT_ERROR_MARGIN;
        rows_height + gaps_height + LANE_SECTION_HEIGHT_SLACK + per_row_margin
    }

    pub(super) fn toggle_repo_group(&mut self, repo_name: SharedString, cx: &mut Context<Self>) {
        let repo_key = repo_name.to_string();
        if self.collapsed_repo_groups.contains(&repo_key) {
            self.collapsed_repo_groups.remove(&repo_key);
        } else {
            self.collapsed_repo_groups.insert(repo_key);
        }
        self.repo_group_animation_versions
            .entry(repo_name.to_string())
            .and_modify(|version| *version += 1)
            .or_insert(1);

        cx.notify();
    }

    pub(super) fn lane_diff_summary(&self, lane_id: u32) -> (u32, u32) {
        self.data
            .changed_files()
            .iter()
            .fold((0, 0), |(adds, dels), file| {
                if file.owner_lane_id == Some(lane_id) {
                    (adds + file.additions, dels + file.deletions)
                } else {
                    (adds, dels)
                }
            })
    }

    pub(super) fn lane_status_label(status: AgentLaneStatus) -> &'static str {
        match status {
            AgentLaneStatus::Completed => "Ready to merge",
            AgentLaneStatus::Blocked => "Merge conflicts",
            AgentLaneStatus::Failed => "Needs changes",
            AgentLaneStatus::Running => "In progress",
            AgentLaneStatus::Queued => "Queued",
        }
    }

    pub(super) fn lane_status_color(&self, status: AgentLaneStatus) -> u32 {
        let colors = self.colors();
        match status {
            AgentLaneStatus::Completed => colors.success,
            AgentLaneStatus::Blocked | AgentLaneStatus::Failed => colors.warning,
            AgentLaneStatus::Running | AgentLaneStatus::Queued => colors.text_muted,
        }
    }

    pub(super) fn render_agents_pane(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let subtitle: SharedString = format!("{} workspaces", self.data.agent_lanes().len()).into();
        let controls = div();

        let mut groups = div().id("agent-lanes-list").flex().flex_col().gap_0();
        for (repo_name, lanes) in self.lane_repo_groups() {
            groups = groups.child(self.render_repo_accordion_section(repo_name, lanes, cx));
        }

        div()
            .id("agents-pane")
            .h_full()
            .w(px(self.left_pane_width))
            .flex_none()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(self.colors().left_panel_bg))
            .border_1()
            .border_color(self.pane_border_color(ActivePane::Left))
            .child(self.render_pane_header(ActivePane::Left, "Agent Lanes", subtitle, controls, cx))
            .child(
                div()
                    .id("agents-pane-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .p_2()
                    .child(groups),
            )
    }

    pub(super) fn render_repo_accordion_section(
        &self,
        repo_name: SharedString,
        lanes: Vec<AgentLane>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = self.colors();
        let is_collapsed = self.is_repo_group_collapsed(repo_name.as_ref());
        let animation_version = self.repo_group_animation_version(repo_name.as_ref());
        let body_max_height = self.repo_group_body_height(lanes.len());
        let chevron_icon = if is_collapsed {
            ICON_CHEVRON_RIGHT
        } else {
            ICON_CHEVRON_DOWN
        };
        let folder_icon = if is_collapsed {
            ICON_FOLDER
        } else {
            ICON_FOLDER_OPEN
        };
        let repo_name_for_click = repo_name.clone();
        let hover_group: SharedString =
            format!("repo-accordion-hover-{}", repo_name.as_ref()).into();
        let header_hover = colors.accent;

        let section = div()
            .pb_1()
            .mb_1()
            .border_b_1()
            .border_color(rgb(colors.border))
            .child(
                div()
                    .rounded_md()
                    .mb_0p5()
                    .h(px(30.0))
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_start()
                    .group(hover_group.clone())
                    .cursor_pointer()
                    .hover(move |style| style.bg(rgb(header_hover)))
                    .on_mouse_move(|_, window, _| window.refresh())
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.toggle_repo_group(repo_name_for_click.clone(), cx)
                        }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .relative()
                                    .w(px(14.0))
                                    .h(px(14.0))
                                    .flex_none()
                                    .child(
                                        div()
                                            .absolute()
                                            .inset_0()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .group_hover(hover_group.clone(), |style| {
                                                style.opacity(0.0)
                                            })
                                            .child(
                                                svg()
                                                    .w(px(14.0))
                                                    .h(px(14.0))
                                                    .path(folder_icon)
                                                    .text_color(rgb(colors.text_muted)),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .absolute()
                                            .inset_0()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .opacity(0.0)
                                            .group_hover(hover_group.clone(), |style| {
                                                style.opacity(1.0)
                                            })
                                            .child(
                                                svg()
                                                    .w(px(14.0))
                                                    .h(px(14.0))
                                                    .path(chevron_icon)
                                                    .text_color(rgb(colors.text_muted)),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(colors.text_primary))
                                    .child(repo_name.clone()),
                            ),
                    ),
            );

        let mut body = div().flex().flex_col().gap_0p5().overflow_hidden();
        for lane in lanes {
            body = body.child(self.render_lane_list_item(&lane, cx));
        }

        let is_collapsed_for_animation = is_collapsed;
        let body_animation_id: SharedString = format!(
            "repo-accordion-body-{}-{}",
            repo_name.as_ref(),
            animation_version
        )
        .into();
        let animated_body = body.with_animation(
            body_animation_id,
            Animation::new(Duration::from_millis(ACCORDION_ANIMATION_MS))
                .with_easing(ease_out_quint()),
            move |this, delta| {
                let progress = if is_collapsed_for_animation {
                    1.0 - delta
                } else {
                    delta
                };
                this.max_h(px(body_max_height * progress.max(0.0)))
                    .opacity(progress)
            },
        );

        section.child(animated_body)
    }

    pub(super) fn render_lane_list_item(
        &self,
        lane: &AgentLane,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = self.colors();
        let is_selected = self.selected_lane_id == Some(lane.id);
        let lane_id = lane.id;
        let (additions, deletions) = self.lane_diff_summary(lane.id);
        let status_label = Self::lane_status_label(lane.status);
        let status_color = self.lane_status_color(lane.status);
        let branch_label: SharedString = lane
            .branch
            .as_ref()
            .strip_prefix("codex/")
            .unwrap_or(lane.branch.as_ref())
            .to_owned()
            .into();

        let row_bg = if is_selected {
            colors.secondary
        } else {
            colors.left_panel_bg
        };
        let row_fg = if is_selected {
            colors.secondary_foreground
        } else {
            colors.text_primary
        };
        let icon_fg = if is_selected {
            colors.secondary_foreground
        } else {
            colors.text_muted
        };
        let sub_fg = if is_selected {
            colors.secondary_foreground
        } else {
            colors.text_muted
        };
        let hover_bg = if is_selected { row_bg } else { colors.accent };

        div()
            .rounded_md()
            .bg(rgb(row_bg))
            .px_2()
            .py_2()
            .flex()
            .items_start()
            .gap_2()
            .cursor_pointer()
            .hover(move |style| style.bg(rgb(hover_bg)))
            .on_mouse_move(|_, window, _| window.refresh())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.select_lane(lane_id, cx)),
            )
            .child(
                div()
                    .w(px(14.0))
                    .h(px(22.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_start()
                    .child(
                        svg()
                            .w(px(14.0))
                            .h(px(14.0))
                            .path(ICON_GIT_BRANCH)
                            .text_color(rgb(icon_fg)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(row_fg))
                                    .child(lane.name.clone()),
                            )
                            .child(
                                div()
                                    .h(px(22.0))
                                    .px_2()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(rgb(colors.border))
                                    .bg(rgb(colors.panel_bg))
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_family(".ZedMono")
                                            .text_color(rgb(colors.success))
                                            .child(format!("+{additions}")),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_family(".ZedMono")
                                            .text_color(rgb(colors.destructive))
                                            .child(format!("-{deletions}")),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div().text_xs().text_color(rgb(sub_fg)).child(branch_label),
                                    )
                                    .child(div().text_xs().text_color(rgb(sub_fg)).child("·"))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(status_color))
                                            .child(status_label),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(sub_fg))
                                    .child(format!("#{}", lane.id)),
                            ),
                    ),
            )
    }
}
