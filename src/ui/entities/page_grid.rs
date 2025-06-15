use std::{collections::BTreeMap, sync::Arc};

use crate::models::PageInfo;
use gpui::{
    Context, EventEmitter, IntoElement, ParentElement, Render, Window, div, prelude::*, px, rgb,
};

#[derive(Clone, Debug)]
pub struct PageSelected {
    pub page_number: u32,
}

pub struct PageGrid {
    pages: Arc<BTreeMap<u32, PageInfo>>,
    selected_page: Option<u32>,
}

impl EventEmitter<PageSelected> for PageGrid {}

impl PageGrid {
    pub fn new(pages: Arc<BTreeMap<u32, PageInfo>>) -> Self {
        Self {
            pages,
            selected_page: None,
        }
    }

    pub fn update_pages(&mut self, pages: Arc<BTreeMap<u32, PageInfo>>, cx: &mut Context<Self>) {
        self.pages = pages;
        cx.notify();
    }

    pub fn select_page(&mut self, page_number: u32, cx: &mut Context<Self>) {
        self.selected_page = Some(page_number);
        cx.emit(PageSelected { page_number });
        cx.notify();
    }
}

impl Render for PageGrid {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut page_grid = div().id("page-grid").flex().flex_wrap().gap_2();

        for (_, page) in self.pages.as_ref() {
            let page_number = page.page_number;
            let is_selected = self.selected_page == Some(page_number);

            page_grid = page_grid.child(
                div()
                    .size(px(80.0))
                    .id(("page", page_number))
                    .bg(page.page_type.color())
                    .when(is_selected, |this| {
                        this.border_2().border_color(rgb(0xffffff))
                    })
                    .when(!is_selected, |this| {
                        this.border_1().border_color(rgb(0x555555))
                    })
                    .rounded_md()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|this| this.opacity(0.7))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(rgb(0xffffff))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                cx.listener(move |this, _event, _window, cx| {
                                    this.select_page(page_number, cx);
                                }),
                            )
                            .child(format!("{}", page.page_number)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .opacity(0.8)
                            .child(page.page_type.short_name()),
                    ),
            );
        }

        div().flex().flex_1().flex_col().p_4().child(page_grid)
    }
}
