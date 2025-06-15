use crate::models::{DatabaseInfo, PageInfo};
use crate::ui::components;
use gpui::{
    Context, IntoElement, ParentElement, Render, Window,
    div, px, prelude::*, rgb
};

pub struct PageSidebar {
    selected_page: Option<u32>,
    pages: Vec<PageInfo>,
    database_info: Option<DatabaseInfo>,
}

impl PageSidebar {
    pub fn new() -> Self {
        Self {
            selected_page: None,
            pages: Vec::new(),
            database_info: None,
        }
    }

    pub fn update_data(
        &mut self,
        selected_page: Option<u32>,
        pages: Vec<PageInfo>,
        database_info: Option<DatabaseInfo>,
        cx: &mut Context<Self>,
    ) {
        self.selected_page = selected_page;
        self.pages = pages;
        self.database_info = database_info;
        cx.notify();
    }

    pub fn set_selected_page(&mut self, page_number: Option<u32>, cx: &mut Context<Self>) {
        self.selected_page = page_number;
        cx.notify();
    }

    pub fn selected_page(&self) -> Option<u32> {
        self.selected_page
    }

    fn get_selected_page_info(&self) -> Option<&PageInfo> {
        if let Some(selected_page_num) = self.selected_page {
            self.pages.iter().find(|p| p.page_number == selected_page_num)
        } else {
            None
        }
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
                    .p_4()
                    .child(if let Some(page) = self.get_selected_page_info() {
                        components::render_page_details(
                            page,
                            self.database_info.as_ref().map(|info| info.header.actual_page_size()),
                        )
                        .into_any_element()
                    } else {
                        div()
                            .text_color(rgb(0xaaaaaa))
                            .child("Select a page to view details")
                            .into_any_element()
                    }),
            )
    }
}

impl Default for PageSidebar {
    fn default() -> Self {
        Self::new()
    }
}