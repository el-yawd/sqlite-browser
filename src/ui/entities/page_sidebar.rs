use std::sync::Arc;
use std::time::Instant;

use crate::models::{DatabaseInfo, PageInfo};
use crate::ui::components;
use gpui::{Context, IntoElement, ParentElement, Render, Window, div, prelude::*, px, rgb};

#[derive(Debug, Clone)]
pub enum SidebarState {
    Empty,
    Loading(u32),
    Loaded(PageInfo),
    Error(String),
}

pub struct PageSidebar {
    pub selected_page: Option<u32>,
    database_info: Option<Arc<DatabaseInfo>>,
    state: SidebarState,
    last_update: Instant,
}

impl PageSidebar {
    pub fn new() -> Self {
        Self {
            selected_page: None,
            database_info: None,
            state: SidebarState::Empty,
            last_update: Instant::now(),
        }
    }

    pub fn update_data(
        &mut self,
        selected_page: Option<u32>,
        database_info: Option<Arc<DatabaseInfo>>,
        cx: &mut Context<Self>,
    ) {
        self.selected_page = selected_page;
        self.database_info = database_info;
        
        // Update state based on the new data
        if selected_page.is_none() {
            self.state = SidebarState::Empty;
        } else if let Some(page_info) = self.get_selected_page_info() {
            self.state = SidebarState::Loaded(page_info.clone());
        } else {
            self.state = SidebarState::Error("Page not found".to_string());
        }
        
        self.last_update = Instant::now();
        cx.notify();
    }

    pub fn set_selected_page(&mut self, page_number: Option<u32>, cx: &mut Context<Self>) {
        self.selected_page = page_number;
        
        match page_number {
            None => {
                self.state = SidebarState::Empty;
            }
            Some(page_num) => {
                // Set loading state immediately for responsive UI
                self.state = SidebarState::Loading(page_num);
                
                // Immediately try to load the page info
                match &self.database_info {
                    Some(db_info) => {
                        if let Some(page_info) = db_info.get_page_info(page_num) {
                            self.state = SidebarState::Loaded(page_info.clone());
                        } else {
                            self.state = SidebarState::Error(format!("Page {} not found", page_num));
                        }
                    }
                    None => {
                        self.state = SidebarState::Error("No database loaded".to_string());
                    }
                }
            }
        }
        
        self.last_update = Instant::now();
        cx.notify();
    }



    fn get_selected_page_info(&self) -> Option<&PageInfo> {
        self.database_info
            .as_ref()?
            .get_page_info(self.selected_page?)
    }

    fn render_loading_indicator(&self) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .gap_2()
            .p_4()
            .child(
                div()
                    .size(px(16.0))
                    .rounded_full()
                    .border_2()
                    .border_color(rgb(0x2563eb))
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0xaaaaaa))
                    .child("Loading page details...")
            )
    }

    fn render_error_state(&self, error: String) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .p_4()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgb(0xef4444))
                    .child("Error")
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0xaaaaaa))
                    .text_center()
                    .child(error)
            )
    }

    fn render_empty_state(&self) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .p_4()
            .child(
                div()
                    .text_color(rgb(0xaaaaaa))
                    .child("Select a page to view details")
            )
    }
}

impl Render for PageSidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("page-sidebar")
            .w(px(300.0))
            .bg(rgb(0x252525))
            .border_l_1()
            .border_color(rgb(0x3e3e3e))
            .flex()
            .flex_col()
            .child(
                div().p_4().border_b_1().border_color(rgb(0x3e3e3e)).child(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(rgb(0xffffff))
                        .child("Page Details"),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0() // Allow shrinking
                    .overflow_hidden()
                    .child(
                        div()
                            .h_full()
                            .child(match &self.state {
                                SidebarState::Empty => self.render_empty_state().into_any_element(),
                                SidebarState::Loading(_page_num) => self.render_loading_indicator().into_any_element(),
                                SidebarState::Loaded(page_info) => {
                                    div()
                                        .p_4()
                                        .max_h_full()
                                        .child(components::render_page_details(
                                            page_info,
                                            self.database_info
                                                .as_ref()
                                                .map(|info| info.header.actual_page_size()),
                                        ))
                                        .into_any_element()
                                }
                                SidebarState::Error(error) => self.render_error_state(error.clone()).into_any_element(),
                            })
                    )
            )
    }
}



impl Default for PageSidebar {
    fn default() -> Self {
        Self::new()
    }
}
