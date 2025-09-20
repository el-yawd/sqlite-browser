use std::{collections::BTreeMap, sync::Arc, time::Instant};

use crate::models::PageInfo;
use gpui::{
    Context, EventEmitter, IntoElement, ParentElement, Render, Window, div, prelude::*, px, rgb,
    MouseDownEvent,
};

#[derive(Clone, Debug)]
pub struct PageSelected {
    pub page_number: u32,
}

#[derive(Debug, Clone)]
pub struct SelectionState {
    pub selected_page: Option<u32>,
    pub selection_timestamp: Instant,
    pub selection_source: SelectionSource,
}

#[derive(Debug, Clone)]
pub enum SelectionSource {
    Mouse,
    Keyboard,
    Programmatic,
}

impl SelectionState {
    pub fn new() -> Self {
        Self {
            selected_page: None,
            selection_timestamp: Instant::now(),
            selection_source: SelectionSource::Programmatic,
        }
    }

    pub fn select_page(&mut self, page_number: u32, source: SelectionSource) {
        self.selected_page = Some(page_number);
        self.selection_timestamp = Instant::now();
        self.selection_source = source;
    }

    pub fn is_selected(&self, page_number: u32) -> bool {
        self.selected_page == Some(page_number)
    }
}

pub struct PageGrid {
    pages: Arc<BTreeMap<u32, PageInfo>>,
    selection_state: SelectionState,
}

impl EventEmitter<PageSelected> for PageGrid {}

impl PageGrid {
    pub fn new(pages: Arc<BTreeMap<u32, PageInfo>>) -> Self {
        Self {
            pages,
            selection_state: SelectionState::new(),
        }
    }

    pub fn update_pages(&mut self, pages: Arc<BTreeMap<u32, PageInfo>>, cx: &mut Context<Self>) {
        self.pages = pages;
        cx.notify();
    }

    pub fn select_page(&mut self, page_number: u32, cx: &mut Context<Self>) {
        // Only update if the selection actually changed to prevent flickering
        if !self.selection_state.is_selected(page_number) {
            self.selection_state.select_page(page_number, SelectionSource::Mouse);
            cx.emit(PageSelected { page_number });
            cx.notify();
        }
    }

    pub fn select_page_programmatically(&mut self, page_number: u32, cx: &mut Context<Self>) {
        self.selection_state.select_page(page_number, SelectionSource::Programmatic);
        cx.emit(PageSelected { page_number });
        cx.notify();
    }

    pub fn get_selected_page(&self) -> Option<u32> {
        self.selection_state.selected_page
    }
}

impl Render for PageGrid {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Create a scrollable container for the page grid
        div()
            .flex()
            .flex_1()
            .flex_col()
            .min_h_0() // Allow shrinking
            .child(
                div()
                    .flex_1()
                    .p_4()
                    .overflow_hidden()
                    .child(
                        div()
                            .id("page-grid-container")
                            .h_full()
                            .child(self.render_page_grid(cx))
                    )
            )
    }
}

impl PageGrid {
    fn render_page_grid(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let pages_per_row = 8; // Adjust based on container width
        let mut rows = Vec::new();
        let mut current_row = Vec::new();
        
        for (_, page) in self.pages.as_ref() {
            current_row.push(page.clone());
            
            if current_row.len() >= pages_per_row {
                rows.push(current_row.clone());
                current_row.clear();
            }
        }
        
        // Add remaining pages as the last row
        if !current_row.is_empty() {
            rows.push(current_row);
        }

        let mut grid_container = div()
            .id("page-grid")
            .flex()
            .flex_col()
            .gap_2()
            .max_h_full();

        for row in rows {
            let mut row_div = div().flex().gap_2().justify_start();
            
            for page in row {
                let page_number = page.page_number;
                let is_selected = self.selection_state.is_selected(page_number);

                row_div = row_div.child(
                    div()
                        .size(px(80.0))
                        .id(("page", page_number))
                        .bg(page.page_type.color())
                        // Enhanced selection visual feedback with immediate response
                        .when(is_selected, |this| {
                            this.border_2()
                                .border_color(rgb(0xffffff))
                                .shadow_lg()
                                .opacity(1.0)
                        })
                        .when(!is_selected, |this| {
                            this.border_1()
                                .border_color(rgb(0x555555))
                                .opacity(0.9)
                        })
                        .rounded_md()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        // Smooth hover transitions with enhanced visual feedback
                        .hover(|this| {
                            this.opacity(0.7)
                                .border_2()
                                .border_color(rgb(0xaaaaaa))
                        })
                        // Mouse event handlers for immediate selection feedback
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                                this.select_page(page_number, cx);
                            }),
                        )
                        .child(
                            div()
                                .text_xs()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(rgb(0xffffff))
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
            
            grid_container = grid_container.child(row_div);
        }

        grid_container
    }
}
