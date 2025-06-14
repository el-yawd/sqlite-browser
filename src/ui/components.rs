use gpui::{IntoElement, ParentElement, InteractiveElement, prelude::*, div, px, rgb};
use crate::models::{DatabaseHeader, DatabaseInfo, PageInfo};

pub fn render_header(
    database_path: Option<&std::path::Path>,
    page_count: usize,
    is_watching: bool,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .p_4()
        .bg(rgb(0x2d2d2d))
        .border_b_1()
        .border_color(rgb(0x3e3e3e))
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .text_xl()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(rgb(0xffffff))
                        .child("SQLite Browser"),
                )
                .when_some(database_path, |this, path| {
                    this.child(div().text_sm().text_color(rgb(0xcccccc)).child(format!(
                        "- {}",
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown")
                    )))
                }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0xaaaaaa))
                        .child(format!("Pages: {}", page_count))
                )
                .when(is_watching, |this| {
                    this.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .size(px(8.0))
                                    .rounded_full()
                                    .bg(rgb(0x4CAF50))
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x4CAF50))
                                    .child("Watching")
                            )
                    )
                })
        )
}

pub fn render_page_grid(
    pages: &[PageInfo],
    selected_page: Option<u32>,
) -> impl IntoElement {
    let mut page_grid = div().flex().flex_wrap().gap_2();

    for page in pages {
        page_grid = page_grid.child(render_page_square(page, selected_page));
    }

    div()
        .flex()
        .flex_1()
        .flex_col()
        .p_4()
        .child(page_grid)
}

pub fn render_page_square(
    page: &PageInfo,
    selected_page: Option<u32>,
) -> impl IntoElement {
    let is_selected = selected_page == Some(page.page_number);

    div()
        .size(px(80.0))
        .bg(page.page_type.color())
        .when(is_selected, |this| this.border_2().border_color(rgb(0xffffff)))
        .when(!is_selected, |this| this.border_1().border_color(rgb(0x555555)))
        .rounded_md()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|this| this.opacity(0.8))
        .id(("page", page.page_number))
        .child(
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(rgb(0xffffff))
                .child(format!("{}", page.page_number))
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xffffff))
                .opacity(0.8)
                .child(page.page_type.short_name())
        )
}

pub fn render_sidebar(
    selected_page: Option<u32>,
    pages: &[PageInfo],
    database_info: Option<&DatabaseInfo>,
) -> impl IntoElement {
    div()
        .w(px(300.0))
        .bg(rgb(0x252525))
        .border_l_1()
        .border_color(rgb(0x3e3e3e))
        .flex()
        .flex_col()
        .child(
            div()
                .p_4()
                .border_b_1()
                .border_color(rgb(0x3e3e3e))
                .child(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(rgb(0xffffff))
                        .child("Page Details")
                )
        )
        .child(
            div()
                .flex_1()
                .p_4()
                .child(
                    if let Some(selected_page_num) = selected_page {
                        if let Some(page) = pages.iter().find(|p| p.page_number == selected_page_num) {
                            render_page_details(page, database_info.map(|info| info.header.actual_page_size())).into_any_element()
                        } else {
                            div()
                                .text_color(rgb(0xaaaaaa))
                                .child("Page not found")
                                .into_any_element()
                        }
                    } else {
                        div()
                            .text_color(rgb(0xaaaaaa))
                            .child("Select a page to view details")
                            .into_any_element()
                    }
                )
        )
        .when_some(database_info, |this, info| {
            this.child(
                div()
                    .border_t_1()
                    .border_color(rgb(0x3e3e3e))
                    .p_4()
                    .child(render_database_info(&info.header))
            )
        })
}

pub fn render_page_details(page: &PageInfo, page_size: Option<usize>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .text_color(rgb(0xffffff))
        .child(
            div()
                .flex()
                .justify_between()
                .child(div().font_weight(gpui::FontWeight::BOLD).child("Page Number:"))
                .child(div().child(format!("{}", page.page_number)))
        )
        .child(
            div()
                .flex()
                .justify_between()
                .child(div().font_weight(gpui::FontWeight::BOLD).child("Page Type:"))
                .child(div().child(page.page_type.name()))
        )
        .child(
            div()
                .flex()
                .justify_between()
                .child(div().font_weight(gpui::FontWeight::BOLD).child("Cell Count:"))
                .child(div().child(format!("{}", page.cell_count)))
        )
        .child(
            div()
                .flex()
                .justify_between()
                .child(div().font_weight(gpui::FontWeight::BOLD).child("Free Space:"))
                .child(div().child(format!("{} bytes", page.free_space)))
        )
        .child(
            div()
                .flex()
                .justify_between()
                .child(div().font_weight(gpui::FontWeight::BOLD).child("Fragmented:"))
                .child(div().child(format!("{} bytes", page.fragmented_bytes)))
        )
        .when_some(page.rightmost_pointer, |this, ptr| {
            this.child(
                div()
                    .flex()
                    .justify_between()
                    .child(div().font_weight(gpui::FontWeight::BOLD).child("Right Pointer:"))
                    .child(div().child(format!("{}", ptr)))
            )
        })
        .when_some(page_size, |this, size| {
            this.child(
                div()
                    .flex()
                    .justify_between()
                    .child(div().font_weight(gpui::FontWeight::BOLD).child("Utilization:"))
                    .child(div().child(format!("{:.1}%", page.utilization_percent(size))))
            )
        })
}

pub fn render_database_info(header: &DatabaseHeader) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(rgb(0xffffff))
                .child("Database Info")
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xcccccc))
                .flex()
                .justify_between()
                .child("Page Size:")
                .child(format!("{}", header.actual_page_size())),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xcccccc))
                .flex()
                .justify_between()
                .child("Total Pages:")
                .child(format!("{}", header.database_size_pages))
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xcccccc))
                .flex()
                .justify_between()
                .child("Schema Version:")
                .child(format!("{}", header.schema_format_number))
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xcccccc))
                .flex()
                .justify_between()
                .child("SQLite Version:")
                .child(format!("{}", header.sqlite_version_number))
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xcccccc))
                .flex()
                .justify_between()
                .child("User Version:")
                .child(format!("{}", header.user_version))
        )
        .when(header.application_id != 0, |this| {
            this.child(
                div()
                    .text_xs()
                    .text_color(rgb(0xcccccc))
                    .flex()
                    .justify_between()
                    .child("App ID:")
                    .child(format!("0x{:08X}", header.application_id))
            )
        })
}

pub fn render_status_message(message: &str, is_error: bool) -> impl IntoElement {
    let message = message.to_string();
    div()
        .p_3()
        .m_2()
        .rounded_md()
        .bg(if is_error { rgb(0x5d1a1a) } else { rgb(0x1a5d2e) })
        .border_1()
        .border_color(if is_error { rgb(0x991b1b) } else { rgb(0x16a34a) })
        .child(
            div()
                .text_sm()
                .text_color(if is_error { rgb(0xfca5a5) } else { rgb(0x86efac) })
                .child(message)
        )
}

pub fn render_loading_indicator() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .p_8()
        .child(
            div()
                .text_color(rgb(0xaaaaaa))
                .child("Loading...")
        )
}

pub fn render_empty_state() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .flex_1()
        .gap_4()
        .child(
            div()
                .text_xl()
                .text_color(rgb(0xaaaaaa))
                .child("No database loaded")
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(0x888888))
                .child("Open a SQLite database file to get started")
        )
}

pub fn render_page_statistics(database_info: &DatabaseInfo) -> impl IntoElement {
    let total_pages = database_info.page_count();
    let page_size = database_info.header.actual_page_size();
    let total_free_space = database_info.total_free_space();
    let avg_utilization = database_info.average_utilization();

    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_4()
        .bg(rgb(0x2a2a2a))
        .rounded_md()
        .child(
            div()
                .text_sm()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(rgb(0xffffff))
                .child("Database Statistics")
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xcccccc))
                .flex()
                .justify_between()
                .child("Total Pages:")
                .child(format!("{}", total_pages))
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xcccccc))
                .flex()
                .justify_between()
                .child("Page Size:")
                .child(format!("{} bytes", page_size))
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xcccccc))
                .flex()
                .justify_between()
                .child("Total Free Space:")
                .child(format!("{} bytes", total_free_space))
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xcccccc))
                .flex()
                .justify_between()
                .child("Avg Utilization:")
                .child(format!("{:.1}%", avg_utilization))
        )
}