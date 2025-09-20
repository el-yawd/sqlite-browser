use gpui::{Context, IntoElement, ParentElement, div, prelude::*, rgb, px};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Unique identifier for status messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusId(u64);

impl StatusId {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        StatusId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Types of status messages with different visual representations
#[derive(Debug, Clone, PartialEq)]
pub enum StatusType {
    Info,
    Success,
    Warning,
    Error,
    Progress(f32), // 0.0 to 1.0
}

/// Optional actions that can be attached to status messages
#[derive(Debug, Clone, PartialEq)]
pub enum StatusAction {
    Retry,
    Dismiss,
    OpenFile,
    ShowDetails,
    Cancel,
}

impl StatusAction {
    fn as_u32(&self) -> u32 {
        match self {
            StatusAction::Retry => 0,
            StatusAction::Dismiss => 1,
            StatusAction::OpenFile => 2,
            StatusAction::ShowDetails => 3,
            StatusAction::Cancel => 4,
        }
    }
}

/// A status message with all its properties
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub id: StatusId,
    pub content: String,
    pub message_type: StatusType,
    pub timestamp: Instant,
    pub dismissible: bool,
    pub auto_dismiss_after: Option<Duration>,
    pub requires_acknowledgment: bool,
    pub action: Option<StatusAction>,
}

impl StatusMessage {
    pub fn new(content: String, message_type: StatusType) -> Self {
        Self {
            id: StatusId::new(),
            content,
            message_type,
            timestamp: Instant::now(),
            dismissible: true,
            auto_dismiss_after: None,
            requires_acknowledgment: false,
            action: None,
        }
    }

    pub fn with_auto_dismiss(mut self, duration: Duration) -> Self {
        self.auto_dismiss_after = Some(duration);
        self
    }

    pub fn with_acknowledgment(mut self) -> Self {
        self.requires_acknowledgment = true;
        self.dismissible = true;
        self.auto_dismiss_after = None;
        self
    }

    pub fn with_action(mut self, action: StatusAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn non_dismissible(mut self) -> Self {
        self.dismissible = false;
        self.auto_dismiss_after = None;
        self
    }
}

/// Manages status messages, queuing, and auto-dismiss functionality
pub struct StatusManager {
    message_queue: VecDeque<StatusMessage>,
    current_message: Option<StatusMessage>,
    // TODO: Add auto_dismiss_tasks in future iteration
}

impl StatusManager {
    pub fn new() -> Self {
        Self {
            message_queue: VecDeque::new(),
            current_message: None,
        }
    }

    /// Show a status message immediately or queue it if another message is currently displayed
    pub fn show_message<T: 'static>(&mut self, message: StatusMessage, cx: &mut Context<T>) {
        if self.current_message.is_none() {
            self.display_message(message, cx);
        } else {
            self.message_queue.push_back(message);
        }
    }

    /// Show an info message with auto-dismiss after 3 seconds
    pub fn show_info<T: 'static>(&mut self, content: String, cx: &mut Context<T>) {
        let message = StatusMessage::new(content, StatusType::Info)
            .with_auto_dismiss(Duration::from_secs(3));
        self.show_message(message, cx);
    }

    /// Show a success message with auto-dismiss after 3 seconds
    pub fn show_success<T: 'static>(&mut self, content: String, cx: &mut Context<T>) {
        let message = StatusMessage::new(content, StatusType::Success)
            .with_auto_dismiss(Duration::from_secs(3));
        self.show_message(message, cx);
    }

    /// Show a warning message that requires user acknowledgment
    pub fn show_warning<T: 'static>(&mut self, content: String, cx: &mut Context<T>) {
        let message = StatusMessage::new(content, StatusType::Warning)
            .with_acknowledgment();
        self.show_message(message, cx);
    }

    /// Show an error message that requires user acknowledgment
    pub fn show_error<T: 'static>(&mut self, content: String, cx: &mut Context<T>) {
        let message = StatusMessage::new(content, StatusType::Error)
            .with_acknowledgment();
        self.show_message(message, cx);
    }

    /// Show a progress message with a progress value (0.0 to 1.0)
    pub fn show_progress<T: 'static>(&mut self, content: String, progress: f32, cx: &mut Context<T>) {
        let message = StatusMessage::new(content, StatusType::Progress(progress.clamp(0.0, 1.0)))
            .non_dismissible()
            .with_action(StatusAction::Cancel);
        self.show_message(message, cx);
    }

    /// Show a progress message without cancel button
    pub fn show_progress_no_cancel<T: 'static>(&mut self, content: String, progress: f32, cx: &mut Context<T>) {
        let message = StatusMessage::new(content, StatusType::Progress(progress.clamp(0.0, 1.0)))
            .non_dismissible();
        self.show_message(message, cx);
    }

    /// Update the progress of the current progress message
    pub fn update_progress<T: 'static>(&mut self, progress: f32, cx: &mut Context<T>) {
        if let Some(ref mut current) = self.current_message {
            if let StatusType::Progress(_) = current.message_type {
                current.message_type = StatusType::Progress(progress.clamp(0.0, 1.0));
                cx.notify();
            }
        }
    }

    /// Manually dismiss the current message
    pub fn dismiss_message<T: 'static>(&mut self, cx: &mut Context<T>) {
        if let Some(_current) = self.current_message.take() {
            // Show next message in queue
            self.show_next_message(cx);
            cx.notify();
        }
    }

    /// Dismiss a specific message by ID
    pub fn dismiss_message_by_id<T: 'static>(&mut self, id: StatusId, cx: &mut Context<T>) {
        // Check if it's the current message
        if let Some(ref current) = self.current_message {
            if current.id == id {
                self.dismiss_message(cx);
                return;
            }
        }

        // Remove from queue if it's there
        self.message_queue.retain(|msg| msg.id != id);
    }

    /// Clear all messages (current and queued)
    pub fn clear_all<T: 'static>(&mut self, cx: &mut Context<T>) {
        self.current_message = None;
        self.message_queue.clear();
        cx.notify();
    }

    /// Get the current message being displayed
    pub fn current_message(&self) -> Option<&StatusMessage> {
        self.current_message.as_ref()
    }

    /// Get the number of messages in the queue
    pub fn queue_length(&self) -> usize {
        self.message_queue.len()
    }

    /// Check if there are any messages (current or queued)
    pub fn has_messages(&self) -> bool {
        self.current_message.is_some() || !self.message_queue.is_empty()
    }

    /// Internal method to display a message and set up auto-dismiss if needed
    fn display_message<T: 'static>(&mut self, message: StatusMessage, cx: &mut Context<T>) {
        let _message_id = message.id;
        
        // TODO: Implement auto-dismiss functionality
        // For now, we'll skip auto-dismiss to keep the implementation simple
        if message.auto_dismiss_after.is_some() {
            // Auto-dismiss will be implemented in a future iteration
        }
        
        self.current_message = Some(message);
        cx.notify();
    }

    /// Internal method to show the next message in the queue
    fn show_next_message<T: 'static>(&mut self, cx: &mut Context<T>) {
        if let Some(next_message) = self.message_queue.pop_front() {
            self.display_message(next_message, cx);
        }
    }

    /// Render the current status message
    pub fn render(&self) -> Option<impl IntoElement> {
        self.current_message.as_ref().map(|message| {
            self.render_status_message(message)
        })
    }

    /// Internal method to render a status message with appropriate styling
    fn render_status_message(&self, message: &StatusMessage) -> impl IntoElement {
        let (bg_color, border_color, text_color) = match message.message_type {
            StatusType::Info => (rgb(0x1e3a8a), rgb(0x3b82f6), rgb(0x93c5fd)),
            StatusType::Success => (rgb(0x14532d), rgb(0x16a34a), rgb(0x86efac)),
            StatusType::Warning => (rgb(0x92400e), rgb(0xd97706), rgb(0xfbbf24)),
            StatusType::Error => (rgb(0x7f1d1d), rgb(0xdc2626), rgb(0xfca5a5)),
            StatusType::Progress(_) => (rgb(0x1e3a8a), rgb(0x3b82f6), rgb(0x93c5fd)),
        };

        div()
            .p_3()
            .m_2()
            .rounded_md()
            .bg(bg_color)
            .border_1()
            .border_color(border_color)
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .text_color(text_color)
                            .child(message.content.clone())
                    )
                    .when_some(self.render_progress_bar(&message.message_type), |this, progress_bar| {
                        this.child(progress_bar)
                    })
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .when_some(message.action.as_ref(), |this, action| {
                        this.child(self.render_action_button(action))
                    })
                    .when(message.dismissible, |this| {
                        this.child(
                            div()
                                .px_2()
                                .py_1()
                                .rounded_sm()
                                .bg(border_color)
                                .hover(|this| this.opacity(0.8))
                                .cursor_pointer()
                                .id(("status-dismiss", message.id.0))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(0xffffff))
                                        .child("✕")
                                )
                        )
                    })
            )
    }

    /// Render progress bar for progress messages
    fn render_progress_bar(&self, message_type: &StatusType) -> Option<impl IntoElement> {
        if let StatusType::Progress(progress) = message_type {
            Some(
                div()
                    .w(px(100.0))
                    .h(px(4.0))
                    .bg(rgb(0x374151))
                    .rounded_full()
                    .child(
                        div()
                            .h_full()
                            .rounded_full()
                            .bg(rgb(0x3b82f6))
                            .w(px(progress * 100.0))
                    )
            )
        } else {
            None
        }
    }

    /// Render action button for messages with actions
    fn render_action_button(&self, action: &StatusAction) -> impl IntoElement {
        let (text, color) = match action {
            StatusAction::Retry => ("Retry", rgb(0x3b82f6)),
            StatusAction::Dismiss => ("Dismiss", rgb(0x6b7280)),
            StatusAction::OpenFile => ("Open File", rgb(0x16a34a)),
            StatusAction::ShowDetails => ("Details", rgb(0xd97706)),
            StatusAction::Cancel => ("Cancel", rgb(0xef4444)),
        };

        div()
            .px_3()
            .py_1()
            .rounded_sm()
            .bg(color)
            .hover(|this| this.opacity(0.8))
            .cursor_pointer()
            .id(("status-action", action.as_u32()))
            .child(
                div()
                    .text_xs()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgb(0xffffff))
                    .child(text)
            )
    }
}

impl Default for StatusManager {
    fn default() -> Self {
        Self::new()
    }
}
#
[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_message_creation() {
        let message = StatusMessage::new("Test message".to_string(), StatusType::Info);
        assert_eq!(message.content, "Test message");
        assert_eq!(message.message_type, StatusType::Info);
        assert!(message.dismissible);
        assert!(message.auto_dismiss_after.is_none());
        assert!(!message.requires_acknowledgment);
        assert!(message.action.is_none());
    }

    #[test]
    fn test_status_message_with_auto_dismiss() {
        let message = StatusMessage::new("Test message".to_string(), StatusType::Success)
            .with_auto_dismiss(Duration::from_secs(3));
        
        assert_eq!(message.auto_dismiss_after, Some(Duration::from_secs(3)));
    }

    #[test]
    fn test_status_message_with_acknowledgment() {
        let message = StatusMessage::new("Test message".to_string(), StatusType::Error)
            .with_acknowledgment();
        
        assert!(message.requires_acknowledgment);
        assert!(message.dismissible);
        assert!(message.auto_dismiss_after.is_none());
    }

    #[test]
    fn test_status_manager_creation() {
        let manager = StatusManager::new();
        assert!(manager.current_message().is_none());
        assert_eq!(manager.queue_length(), 0);
        assert!(!manager.has_messages());
    }

    #[test]
    fn test_status_action_as_u32() {
        assert_eq!(StatusAction::Retry.as_u32(), 0);
        assert_eq!(StatusAction::Dismiss.as_u32(), 1);
        assert_eq!(StatusAction::OpenFile.as_u32(), 2);
        assert_eq!(StatusAction::ShowDetails.as_u32(), 3);
        assert_eq!(StatusAction::Cancel.as_u32(), 4);
    }
}