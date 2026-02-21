use super::*;
use std::{ops::Range, time::Instant};

const COMPOSER_LINE_HEIGHT: f32 = 20.0;
const COMPOSER_CARET_WIDTH: f32 = 1.0;
const COMPOSER_CARET_BLINK_MS: u128 = 530;
const COMPOSER_DRAG_THRESHOLD: f32 = 3.0;

#[derive(Debug, Clone)]
pub(super) struct ExpandableTextAreaSpec {
    pub id_suffix: SharedString,
    pub placeholder: SharedString,
    pub min_lines: usize,
    pub max_lines: usize,
}

impl ExpandableTextAreaSpec {
    pub(super) fn new(
        id_suffix: impl Into<SharedString>,
        placeholder: impl Into<SharedString>,
        min_lines: usize,
        max_lines: usize,
    ) -> Self {
        Self {
            id_suffix: id_suffix.into(),
            placeholder: placeholder.into(),
            min_lines,
            max_lines,
        }
    }
}

struct ComposerTextElement {
    input: Entity<AppShell>,
}

struct ComposerPrepaintState {
    wrapped_lines: Vec<WrappedLine>,
    cursor: Option<PaintQuad>,
    line_height: Pixels,
}

impl AppShell {
    pub(super) fn render_expandable_text_area(
        &mut self,
        spec: ExpandableTextAreaSpec,
        footer: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = self.colors();
        if self.composer_placeholder != spec.placeholder {
            self.composer_placeholder = spec.placeholder.clone();
        }
        let min_lines = spec.min_lines.max(1);
        let max_lines = spec.max_lines.max(min_lines);
        let total_lines = self.composer_total_visual_lines.max(1);
        let visible_lines = total_lines.clamp(min_lines, max_lines);
        let viewport_height = px((visible_lines as f32 * COMPOSER_LINE_HEIGHT) + 8.0);
        let textarea_id: SharedString = format!("expandable-textarea-{}", spec.id_suffix).into();

        div()
            .id(textarea_id)
            .rounded_lg()
            .border_1()
            .border_color(rgb(colors.border_strong))
            .bg(rgb(colors.card_bg))
            .px_3()
            .pt_2()
            .pb_2()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .id("middle-chat-composer-input")
                    .h(viewport_height)
                    .w_full()
                    .cursor(CursorStyle::IBeam)
                    .track_focus(&self.composer_focus_handle)
                    .line_height(px(COMPOSER_LINE_HEIGHT))
                    .text_sm()
                    .text_color(rgb(colors.text_primary))
                    .key_context("middle-chat-composer")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, window, cx| {
                            this.active_pane = ActivePane::Middle;
                            window.focus(&this.composer_focus_handle, cx);
                            let clicked_index =
                                this.composer_index_for_window_point(event.position, window);
                            if event.modifiers.shift {
                                let anchor = this
                                    .composer_mouse_selection_anchor
                                    .unwrap_or_else(|| this.composer_caret());
                                this.composer_set_selection(anchor, clicked_index);
                            } else {
                                this.composer_set_selection(clicked_index, clicked_index);
                            }
                            this.composer_mouse_selecting = true;
                            this.composer_mouse_down_point = Some(event.position);
                            this.composer_marked_range = None;
                            this.composer_restart_caret_blink();
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    )
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        let mut handled = false;
                        let modifiers = event.keystroke.modifiers;

                        match event.keystroke.key.as_str() {
                            "enter" if !event.is_held => {
                                window.prevent_default();
                                if event.keystroke.modifiers.shift {
                                    this.composer_insert_newline(cx);
                                } else {
                                    this.submit_composer_message(cx);
                                }
                                handled = true;
                            }
                            "backspace" => {
                                window.prevent_default();
                                this.composer_delete_backward(cx);
                                handled = true;
                            }
                            "delete" => {
                                window.prevent_default();
                                this.composer_delete_forward(cx);
                                handled = true;
                            }
                            "left" => {
                                window.prevent_default();
                                if modifiers.secondary() {
                                    this.composer_move_caret_to_line_start(cx);
                                } else {
                                    this.composer_move_caret_left(cx);
                                }
                                handled = true;
                            }
                            "right" => {
                                window.prevent_default();
                                if modifiers.secondary() {
                                    this.composer_move_caret_to_line_end(cx);
                                } else {
                                    this.composer_move_caret_right(cx);
                                }
                                handled = true;
                            }
                            "up" => {
                                window.prevent_default();
                                if modifiers.secondary() {
                                    this.composer_move_caret_to(0, cx);
                                } else {
                                    this.composer_move_caret_up(cx);
                                }
                                handled = true;
                            }
                            "down" => {
                                window.prevent_default();
                                if modifiers.secondary() {
                                    let end = this.composer_content.len();
                                    this.composer_move_caret_to(end, cx);
                                } else {
                                    this.composer_move_caret_down(cx);
                                }
                                handled = true;
                            }
                            "home" => {
                                window.prevent_default();
                                this.composer_move_caret_to_line_start(cx);
                                handled = true;
                            }
                            "end" => {
                                window.prevent_default();
                                this.composer_move_caret_to_line_end(cx);
                                handled = true;
                            }
                            "a" if modifiers.secondary() && !event.is_held => {
                                window.prevent_default();
                                this.composer_select_all(cx);
                                handled = true;
                            }
                            _ => {}
                        }

                        if handled {
                            // Keep handled keys from bubbling to shell-level handlers.
                            cx.stop_propagation();
                        }
                    }))
                    .overflow_y_scroll()
                    .track_scroll(&self.composer_input_scroll)
                    .child(
                        div()
                            .w_full()
                            .py_1()
                            .child(ComposerTextElement { input: cx.entity() }),
                    ),
            )
            .child(footer)
    }

    pub(super) fn composer_is_empty(&self) -> bool {
        self.composer_content.trim().is_empty()
    }

    fn composer_restart_caret_blink(&mut self) {
        self.composer_caret_blink_started_at = Instant::now();
    }

    fn composer_caret_is_visible(&self, now: Instant) -> bool {
        let elapsed_ms = now
            .saturating_duration_since(self.composer_caret_blink_started_at)
            .as_millis();
        (elapsed_ms / COMPOSER_CARET_BLINK_MS).is_multiple_of(2)
    }

    fn composer_normalized_selected_range(&self) -> Range<usize> {
        let len = self.composer_content.len();
        let start = self.composer_selected_range.start.min(len);
        let end = self.composer_selected_range.end.min(len);
        if start <= end {
            start..end
        } else {
            end..start
        }
    }

    fn composer_set_selection(&mut self, anchor: usize, caret: usize) {
        let len = self.composer_content.len();
        let anchor = anchor.min(len);
        let caret = caret.min(len);
        self.composer_mouse_selection_anchor = Some(anchor);
        self.composer_marked_range = None;
        self.composer_selected_range = if anchor <= caret {
            anchor..caret
        } else {
            caret..anchor
        };
    }

    fn composer_index_for_window_point(
        &self,
        window_point: Point<Pixels>,
        window: &mut Window,
    ) -> usize {
        let content = self.composer_content.as_ref();
        if content.is_empty() {
            return 0;
        }
        let Some(bounds) = self.composer_last_bounds else {
            return content.len();
        };

        let mut local_x = window_point.x - bounds.left();
        if local_x < px(0.0) {
            local_x = px(0.0);
        }

        let mut local_y = window_point.y - bounds.top();
        if local_y < px(0.0) {
            local_y = px(0.0);
        }

        let run = TextRun {
            len: content.len(),
            font: window.text_style().font(),
            color: window.text_style().color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let wrapped_lines = window
            .text_system()
            .shape_text(
                self.composer_content.clone(),
                window.text_style().font_size.to_pixels(window.rem_size()),
                &[run],
                Some(bounds.size.width),
                None,
            )
            .map(|lines| lines.into_iter().collect::<Vec<_>>())
            .unwrap_or_default();

        let line_height = px(COMPOSER_LINE_HEIGHT);
        let mut segment_start = 0usize;
        let mut y_offset = px(0.0);
        let segment_count = content.split('\n').count().max(1);

        for (segment_ix, segment) in content.split('\n').enumerate() {
            let segment_len = segment.len();
            let segment_end = segment_start + segment_len;
            let is_last_segment = segment_ix + 1 == segment_count;
            let wrapped = wrapped_lines.get(segment_ix);
            let line_count = wrapped
                .map(|line| line.wrap_boundaries().len() + 1)
                .unwrap_or(1);
            let segment_height = line_height * line_count as f32;

            if local_y <= y_offset + segment_height || is_last_segment {
                if let Some(wrapped) = wrapped {
                    let max_local_y = (segment_height - px(0.001)).max(px(0.0));
                    let mut y_in_segment = local_y - y_offset;
                    if y_in_segment < px(0.0) {
                        y_in_segment = px(0.0);
                    }
                    if y_in_segment > max_local_y {
                        y_in_segment = max_local_y;
                    }

                    let local_index = wrapped
                        .closest_index_for_position(point(local_x, y_in_segment), line_height)
                        .unwrap_or_else(|index| index)
                        .min(segment_len);
                    return (segment_start + local_index).min(content.len());
                }

                return segment_start.min(content.len());
            }

            y_offset += segment_height;
            segment_start = segment_end + 1;
        }

        content.len()
    }

    pub(super) fn composer_insert_newline(&mut self, cx: &mut Context<Self>) {
        let selected = self.composer_normalized_selected_range();
        self.composer_content =
            (self.composer_content[0..selected.start].to_owned()
                + "\n"
                + &self.composer_content[selected.end..])
                .into();
        let caret = selected.start + 1;
        self.composer_selected_range = caret..caret;
        self.composer_mouse_selection_anchor = Some(caret);
        self.composer_mouse_selecting = false;
        self.composer_mouse_down_point = None;
        self.composer_marked_range = None;
        self.composer_restart_caret_blink();
        self.composer_input_scroll.scroll_to_bottom();
        cx.notify();
    }

    pub(super) fn submit_composer_message(&mut self, cx: &mut Context<Self>) {
        if self.composer_is_empty() {
            return;
        }

        self.chat_messages.push(ChatMessage {
            author: "You".into(),
            text: self.composer_content.clone(),
            outgoing: true,
        });
        self.composer_content = "".into();
        self.composer_selected_range = 0..0;
        self.composer_mouse_selection_anchor = Some(0);
        self.composer_mouse_selecting = false;
        self.composer_mouse_down_point = None;
        self.composer_marked_range = None;
        self.composer_restart_caret_blink();
        self.composer_total_visual_lines = 1;
        self.composer_input_scroll.scroll_to_bottom();
        self.chat_thread_scroll.scroll_to_bottom();
        self.status_text = "Sent message to agent".into();
        cx.notify();
    }

    fn composer_caret(&self) -> usize {
        self.composer_normalized_selected_range()
            .end
            .min(self.composer_content.len())
    }

    fn composer_prev_char_boundary(text: &str, index: usize) -> usize {
        text[..index]
            .char_indices()
            .next_back()
            .map(|(offset, _)| offset)
            .unwrap_or(0)
    }

    fn composer_next_char_boundary(text: &str, index: usize) -> usize {
        if index >= text.len() {
            return text.len();
        }

        let next = text[index..]
            .chars()
            .next()
            .map(|ch| index + ch.len_utf8())
            .unwrap_or(index);
        next.min(text.len())
    }

    fn composer_offset_for_char_column(
        text: &str,
        line_start: usize,
        line_end: usize,
        column_chars: usize,
    ) -> usize {
        let mut index = line_start;
        let mut remaining = column_chars;

        for ch in text[line_start..line_end].chars() {
            if remaining == 0 {
                break;
            }
            index += ch.len_utf8();
            remaining -= 1;
        }

        index.min(line_end)
    }

    fn composer_move_caret_to(&mut self, index: usize, cx: &mut Context<Self>) {
        let index = index.min(self.composer_content.len());
        self.composer_mouse_selecting = false;
        self.composer_mouse_selection_anchor = Some(index);
        self.composer_mouse_down_point = None;
        self.composer_selected_range = index..index;
        self.composer_marked_range = None;
        self.composer_restart_caret_blink();
        cx.notify();
    }

    fn composer_select_all(&mut self, cx: &mut Context<Self>) {
        let len = self.composer_content.len();
        self.composer_mouse_selecting = false;
        self.composer_mouse_selection_anchor = Some(0);
        self.composer_mouse_down_point = None;
        self.composer_selected_range = 0..len;
        self.composer_marked_range = None;
        self.composer_restart_caret_blink();
        cx.notify();
    }

    fn composer_delete_range(&mut self, range: Range<usize>, cx: &mut Context<Self>) {
        if range.start >= range.end || range.end > self.composer_content.len() {
            return;
        }

        self.composer_content =
            (self.composer_content[0..range.start].to_owned() + &self.composer_content[range.end..])
                .into();
        self.composer_selected_range = range.start..range.start;
        self.composer_mouse_selection_anchor = Some(range.start);
        self.composer_mouse_selecting = false;
        self.composer_mouse_down_point = None;
        self.composer_marked_range = None;
        self.composer_restart_caret_blink();
        self.composer_input_scroll.scroll_to_bottom();
        cx.notify();
    }

    fn composer_delete_backward(&mut self, cx: &mut Context<Self>) {
        let selected = self.composer_normalized_selected_range();
        if selected.start != selected.end {
            self.composer_delete_range(selected, cx);
            return;
        }

        let caret = self.composer_caret();
        if caret == 0 {
            return;
        }

        let delete_start = Self::composer_prev_char_boundary(self.composer_content.as_ref(), caret);
        self.composer_delete_range(delete_start..caret, cx);
    }

    fn composer_delete_forward(&mut self, cx: &mut Context<Self>) {
        let selected = self.composer_normalized_selected_range();
        if selected.start != selected.end {
            self.composer_delete_range(selected, cx);
            return;
        }

        let caret = self.composer_caret();
        if caret >= self.composer_content.len() {
            return;
        }

        let delete_end = Self::composer_next_char_boundary(self.composer_content.as_ref(), caret);
        self.composer_delete_range(caret..delete_end, cx);
    }

    fn composer_move_caret_left(&mut self, cx: &mut Context<Self>) {
        let selected = self.composer_normalized_selected_range();
        if selected.start != selected.end {
            self.composer_move_caret_to(selected.start, cx);
            return;
        }

        let caret = self.composer_caret();
        let previous = Self::composer_prev_char_boundary(self.composer_content.as_ref(), caret);
        self.composer_move_caret_to(previous, cx);
    }

    fn composer_move_caret_right(&mut self, cx: &mut Context<Self>) {
        let selected = self.composer_normalized_selected_range();
        if selected.start != selected.end {
            self.composer_move_caret_to(selected.end, cx);
            return;
        }

        let caret = self.composer_caret();
        let next = Self::composer_next_char_boundary(self.composer_content.as_ref(), caret);
        self.composer_move_caret_to(next, cx);
    }

    fn composer_move_caret_to_line_start(&mut self, cx: &mut Context<Self>) {
        let content = self.composer_content.as_ref();
        let caret = self.composer_caret();
        let line_start = content[..caret].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
        self.composer_move_caret_to(line_start, cx);
    }

    fn composer_move_caret_to_line_end(&mut self, cx: &mut Context<Self>) {
        let content = self.composer_content.as_ref();
        let caret = self.composer_caret();
        let line_end = content[caret..]
            .find('\n')
            .map(|offset| caret + offset)
            .unwrap_or(content.len());
        self.composer_move_caret_to(line_end, cx);
    }

    fn composer_move_caret_up(&mut self, cx: &mut Context<Self>) {
        let content = self.composer_content.as_ref();
        let caret = self.composer_caret();

        let current_line_start = content[..caret].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
        if current_line_start == 0 {
            self.composer_move_caret_to(0, cx);
            return;
        }

        let current_col_chars = content[current_line_start..caret].chars().count();
        let previous_line_end = current_line_start - 1;
        let previous_line_start = content[..previous_line_end]
            .rfind('\n')
            .map(|idx| idx + 1)
            .unwrap_or(0);
        let target = Self::composer_offset_for_char_column(
            content,
            previous_line_start,
            previous_line_end,
            current_col_chars,
        );
        self.composer_move_caret_to(target, cx);
    }

    fn composer_move_caret_down(&mut self, cx: &mut Context<Self>) {
        let content = self.composer_content.as_ref();
        let caret = self.composer_caret();

        let current_line_start = content[..caret].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
        let current_line_end = content[caret..]
            .find('\n')
            .map(|offset| caret + offset)
            .unwrap_or(content.len());
        if current_line_end >= content.len() {
            self.composer_move_caret_to(content.len(), cx);
            return;
        }

        let current_col_chars = content[current_line_start..caret].chars().count();
        let next_line_start = current_line_end + 1;
        let next_line_end = content[next_line_start..]
            .find('\n')
            .map(|offset| next_line_start + offset)
            .unwrap_or(content.len());
        let target = Self::composer_offset_for_char_column(
            content,
            next_line_start,
            next_line_end,
            current_col_chars,
        );
        self.composer_move_caret_to(target, cx);
    }

    fn composer_offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.composer_content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    fn composer_offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in self.composer_content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    fn composer_range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.composer_offset_to_utf16(range.start)..self.composer_offset_to_utf16(range.end)
    }

    fn composer_range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.composer_offset_from_utf16(range_utf16.start)
            ..self.composer_offset_from_utf16(range_utf16.end)
    }

    fn wrap_boundary_index_and_x(wrapped: &WrappedLine, boundary_ix: usize) -> (usize, Pixels) {
        let Some(boundary) = wrapped.wrap_boundaries().get(boundary_ix).copied() else {
            return (wrapped.len(), wrapped.unwrapped_layout.width);
        };

        if let Some(run) = wrapped.runs().get(boundary.run_ix) {
            if let Some(glyph) = run.glyphs.get(boundary.glyph_ix) {
                return (glyph.index, glyph.position.x);
            }
        }

        (wrapped.len(), wrapped.unwrapped_layout.width)
    }

    fn wrapped_line_cursor_line_and_x(wrapped: &WrappedLine, index: usize) -> (usize, Pixels) {
        let mut line_start_x = px(0.0);
        let line_count = wrapped.wrap_boundaries().len() + 1;
        let clamped_index = index.min(wrapped.len());

        for line_ix in 0..line_count {
            let line_end_index = if line_ix < wrapped.wrap_boundaries().len() {
                Self::wrap_boundary_index_and_x(wrapped, line_ix).0
            } else {
                wrapped.len()
            };

            if clamped_index < line_end_index || line_ix == line_count - 1 {
                let unwrapped_x = wrapped.unwrapped_layout.x_for_index(clamped_index);
                return (line_ix, unwrapped_x - line_start_x);
            }

            if line_ix < wrapped.wrap_boundaries().len() {
                let (_, next_line_start_x) = Self::wrap_boundary_index_and_x(wrapped, line_ix);
                line_start_x = next_line_start_x;
            }
        }

        (0, px(0.0))
    }

    fn composer_local_point_for_index(
        content: &str,
        wrapped_lines: &[WrappedLine],
        index: usize,
        line_height: Pixels,
    ) -> Point<Pixels> {
        let mut segment_start = 0usize;
        let mut y_offset = px(0.0);
        let segment_count = content.split('\n').count().max(1);

        for (segment_ix, segment) in content.split('\n').enumerate() {
            let segment_len = segment.len();
            let segment_end = segment_start + segment_len;
            let is_last_segment = segment_ix + 1 == segment_count;

            if index <= segment_end || is_last_segment {
                let local_index = index.saturating_sub(segment_start).min(segment_len);
                if let Some(wrapped) = wrapped_lines.get(segment_ix) {
                    let (line_ix, line_x) = Self::wrapped_line_cursor_line_and_x(wrapped, local_index);
                    return point(line_x, y_offset + (line_height * line_ix as f32));
                }

                return point(px(0.0), y_offset);
            }

            let consumed_lines = wrapped_lines
                .get(segment_ix)
                .map(|wrapped| wrapped.wrap_boundaries().len() + 1)
                .unwrap_or(1);
            y_offset += line_height * consumed_lines as f32;
            segment_start = segment_end + 1;
        }

        point(px(0.0), y_offset)
    }

    fn count_visual_lines(wrapped_lines: &[WrappedLine]) -> usize {
        wrapped_lines
            .iter()
            .map(|wrapped| wrapped.wrap_boundaries().len() + 1)
            .sum::<usize>()
            .max(1)
    }
}

impl EntityInputHandler for AppShell {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.composer_range_from_utf16(&range_utf16);
        actual_range.replace(self.composer_range_to_utf16(&range));
        Some(self.composer_content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let selected = self.composer_normalized_selected_range();
        Some(UTF16Selection {
            range: self.composer_range_to_utf16(&selected),
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.composer_marked_range
            .as_ref()
            .map(|range| self.composer_range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.composer_marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.composer_range_from_utf16(range_utf16))
            .or(self.composer_marked_range.clone())
            .unwrap_or_else(|| self.composer_normalized_selected_range());

        self.composer_content =
            (self.composer_content[0..range.start].to_owned()
                + new_text
                + &self.composer_content[range.end..])
                .into();
        let end = range.start + new_text.len();
        self.composer_selected_range = end..end;
        self.composer_mouse_selection_anchor = Some(end);
        self.composer_mouse_selecting = false;
        self.composer_marked_range = None;
        self.composer_restart_caret_blink();
        self.composer_input_scroll.scroll_to_bottom();
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.composer_range_from_utf16(range_utf16))
            .or(self.composer_marked_range.clone())
            .unwrap_or_else(|| self.composer_normalized_selected_range());

        self.composer_content =
            (self.composer_content[0..range.start].to_owned()
                + new_text
                + &self.composer_content[range.end..])
                .into();

        if new_text.is_empty() {
            self.composer_marked_range = None;
        } else {
            self.composer_marked_range = Some(range.start..range.start + new_text.len());
        }

        self.composer_selected_range = new_selected_range_utf16
            .as_ref()
            .map(|selection_utf16| self.composer_range_from_utf16(selection_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.start)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());

        self.composer_mouse_selection_anchor = Some(self.composer_selected_range.end);
        self.composer_mouse_selecting = false;
        self.composer_input_scroll.scroll_to_bottom();
        self.composer_restart_caret_blink();
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.composer_range_from_utf16(&range_utf16);
        let index = range.end.min(self.composer_content.len());
        let run = TextRun {
            len: self.composer_content.len(),
            font: window.text_style().font(),
            color: window.text_style().color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let wrapped_lines = window
            .text_system()
            .shape_text(
                self.composer_content.clone(),
                window.text_style().font_size.to_pixels(window.rem_size()),
                &[run],
                Some(bounds.size.width),
                None,
            )
            .map(|lines| lines.into_iter().collect::<Vec<_>>())
            .unwrap_or_default();
        let local = Self::composer_local_point_for_index(
            self.composer_content.as_ref(),
            &wrapped_lines,
            index,
            px(COMPOSER_LINE_HEIGHT),
        );

        Some(Bounds::new(
            point(bounds.left() + local.x, bounds.top() + local.y),
            size(px(COMPOSER_CARET_WIDTH), px(COMPOSER_LINE_HEIGHT)),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let index = self.composer_index_for_window_point(point, window);
        Some(self.composer_offset_to_utf16(index))
    }
}

impl IntoElement for ComposerTextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ComposerTextElement {
    type RequestLayoutState = ();
    type PrepaintState = ComposerPrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        let lines = self.input.read(cx).composer_total_visual_lines.max(1);
        style.size.height = px(lines as f32 * COMPOSER_LINE_HEIGHT).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let line_height = px(COMPOSER_LINE_HEIGHT);
        let (
            content,
            placeholder,
            selected_range,
            muted_text_color,
            selection_bg_color,
            focused,
            caret_visible,
        ) = {
            let shell = self.input.read(cx);
            let now = Instant::now();
            (
                shell.composer_content.clone(),
                shell.composer_placeholder.clone(),
                shell.composer_normalized_selected_range(),
                shell.colors().text_muted,
                Hsla::from(rgb(shell.colors().primary)).opacity(0.42),
                shell.composer_focus_handle.is_focused(window),
                shell.composer_caret_is_visible(now),
            )
        };
        let is_placeholder = content.is_empty();
        let text_to_shape = if is_placeholder { placeholder } else { content.clone() };
        let mut text_color = window.text_style().color;
        if is_placeholder {
            text_color = rgb(muted_text_color).into();
        }

        let mut runs = Vec::new();
        if is_placeholder || selected_range.start == selected_range.end {
            runs.push(TextRun {
                len: text_to_shape.len(),
                font: window.text_style().font(),
                color: text_color,
                background_color: None,
                underline: None,
                strikethrough: None,
            });
        } else {
            if selected_range.start > 0 {
                runs.push(TextRun {
                    len: selected_range.start,
                    font: window.text_style().font(),
                    color: text_color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                });
            }
            let selected_len = selected_range.end.saturating_sub(selected_range.start);
            if selected_len > 0 {
                runs.push(TextRun {
                    len: selected_len,
                    font: window.text_style().font(),
                    color: text_color,
                    background_color: Some(selection_bg_color),
                    underline: None,
                    strikethrough: None,
                });
            }
            if selected_range.end < text_to_shape.len() {
                runs.push(TextRun {
                    len: text_to_shape.len() - selected_range.end,
                    font: window.text_style().font(),
                    color: text_color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                });
            }
        }

        let wrapped_lines = window
            .text_system()
            .shape_text(
                text_to_shape.clone(),
                window.text_style().font_size.to_pixels(window.rem_size()),
                &runs,
                Some(bounds.size.width),
                None,
            )
            .map(|lines| lines.into_iter().collect::<Vec<_>>())
            .unwrap_or_default();

        let visual_lines = AppShell::count_visual_lines(&wrapped_lines);
        let caret_index = if is_placeholder {
            0
        } else {
            selected_range.end.min(content.len())
        };
        let caret_local = if is_placeholder {
            point(px(0.0), px(0.0))
        } else {
            AppShell::composer_local_point_for_index(
                content.as_ref(),
                &wrapped_lines,
                caret_index,
                line_height,
            )
        };

        if focused {
            window.request_animation_frame();
        }

        let cursor = if focused && selected_range.start == selected_range.end && caret_visible {
            Some(fill(
                Bounds::new(
                    point(bounds.left() + caret_local.x, bounds.top() + caret_local.y),
                    size(px(COMPOSER_CARET_WIDTH), line_height),
                ),
                text_color,
            ))
        } else {
            None
        };

        self.input.update(cx, |this, cx| {
            let mut should_notify = false;
            if this.composer_total_visual_lines != visual_lines {
                this.composer_total_visual_lines = visual_lines;
                should_notify = true;
            }

            this.composer_last_bounds = Some(bounds);

            if should_notify {
                cx.notify();
            }
        });

        ComposerPrepaintState {
            wrapped_lines,
            cursor,
            line_height,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).composer_focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );

        let input_for_drag = self.input.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble {
                return;
            }

            input_for_drag.update(cx, |this, cx| {
                if !this.composer_mouse_selecting {
                    return;
                }
                if !event.dragging() {
                    this.composer_mouse_selecting = false;
                    this.composer_mouse_down_point = None;
                    cx.notify();
                    return;
                }

                let drag_start = this.composer_mouse_down_point.unwrap_or(event.position);
                let delta = event.position - drag_start;
                if delta.x.abs() < px(COMPOSER_DRAG_THRESHOLD)
                    && delta.y.abs() < px(COMPOSER_DRAG_THRESHOLD)
                {
                    return;
                }

                let dragged_index = this.composer_index_for_window_point(event.position, window);
                let anchor = this.composer_mouse_selection_anchor.unwrap_or(dragged_index);
                let previous = this.composer_normalized_selected_range();
                this.composer_set_selection(anchor, dragged_index);
                if this.composer_normalized_selected_range() != previous {
                    this.composer_restart_caret_blink();
                    cx.notify();
                }
            });
        });

        let input_for_release = self.input.clone();
        window.on_mouse_event(move |event: &MouseUpEvent, phase, _window, cx| {
            if phase != DispatchPhase::Bubble || event.button != MouseButton::Left {
                return;
            }

            input_for_release.update(cx, |this, cx| {
                if this.composer_mouse_selecting {
                    this.composer_mouse_selecting = false;
                }
                this.composer_mouse_down_point = None;
                cx.notify();
            });
        });

        let mut y = bounds.top();
        for wrapped in &prepaint.wrapped_lines {
            let _ = wrapped.paint_background(
                point(bounds.left(), y),
                prepaint.line_height,
                TextAlign::Left,
                None,
                window,
                cx,
            );
            let _ = wrapped.paint(
                point(bounds.left(), y),
                prepaint.line_height,
                TextAlign::Left,
                None,
                window,
                cx,
            );
            y += prepaint.line_height * (wrapped.wrap_boundaries().len() as f32 + 1.0);
        }

        if let Some(cursor) = prepaint.cursor.take() {
            window.paint_quad(cursor);
        }
    }
}
