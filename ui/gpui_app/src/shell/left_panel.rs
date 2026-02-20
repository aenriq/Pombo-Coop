use super::*;
use crate::mock_data::{AgentLane, AgentLaneStatus};

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

    pub(super) fn toggle_repo_group(&mut self, repo_name: SharedString, cx: &mut Context<Self>) {
        let repo_key = repo_name.to_string();
        if self.collapsed_repo_groups.contains(&repo_key) {
            self.collapsed_repo_groups.remove(&repo_key);
        } else {
            self.collapsed_repo_groups.insert(repo_key);
        }

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
        let chevron = if is_collapsed { ">" } else { "v" };
        let repo_name_for_click = repo_name.clone();
        let header_hover = colors.card_bg;

        let mut section = div()
            .pb_1()
            .mb_1()
            .border_b_1()
            .border_color(rgb(colors.border))
            .child(
                div()
                    .h(px(30.0))
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_between()
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
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(colors.text_primary))
                            .child(repo_name),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(colors.text_muted))
                            .child(chevron),
                    ),
            );

        if !is_collapsed {
            let mut body = div().flex().flex_col().gap_0p5().pl_1();
            for lane in lanes {
                body = body.child(self.render_lane_list_item(&lane, cx));
            }
            section = section.child(body);
        }

        section
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
        let sub_fg = if is_selected {
            colors.secondary_foreground
        } else {
            colors.text_muted
        };
        let hover_bg = if is_selected { row_bg } else { colors.card_bg };

        div()
            .rounded_md()
            .bg(rgb(row_bg))
            .px_2()
            .py_2()
            .flex()
            .flex_col()
            .gap_1()
            .cursor_pointer()
            .hover(move |style| style.bg(rgb(hover_bg)))
            .on_mouse_move(|_, window, _| window.refresh())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.select_lane(lane_id, cx)),
            )
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
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
                            .child(div().text_sm().text_color(rgb(sub_fg)).child(branch_label))
                            .child(div().text_sm().text_color(rgb(sub_fg)).child("·"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(status_color))
                                    .child(status_label),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(sub_fg))
                            .child(format!("#{}", lane.id)),
                    ),
            )
    }
}
