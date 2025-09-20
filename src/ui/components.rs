use crate::models::{DatabaseHeader, DatabaseInfo, PageInfo};
use gpui::{InteractiveElement, IntoElement, ParentElement, div, prelude::*, px, rgb};

/// Validates page data for consistency and safety
fn validate_page_data(page: &PageInfo, page_size: Option<usize>) -> Result<(), String> {
    if let Some(size) = page_size {
        if size == 0 {
            return Err("Page size cannot be zero".to_string());
        }
        
        let size_u16 = size as u16;
        
        if page.free_space > size_u16 {
            return Err(format!(
                "Free space ({} bytes) exceeds page size ({} bytes)", 
                page.free_space, size
            ));
        }
        
        let total_overhead = page.free_space.saturating_add(page.fragmented_bytes as u16);
        if total_overhead > size_u16 {
            return Err(format!(
                "Total overhead ({} bytes) exceeds page size ({} bytes)", 
                total_overhead, size
            ));
        }
    }
    
    Ok(())
}

/// Renders page details when there's invalid page data
fn render_page_details_error(page: &PageInfo, error_message: &str) -> impl IntoElement {
    let error_message = error_message.to_string();
    div()
        .flex()
        .flex_col()
        .gap_3()
        .text_color(rgb(0xffffff))
        .child(
            // Page header with color indicator
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .size(px(16.0))
                        .rounded_full()
                        .bg(page.page_type.color()),
                )
                .child(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(format!("Page {}", page.page_number)),
                ),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Page Type:"),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().child(page.page_type.name()))
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0xaaaaaa))
                                .child(format!("({})", page.page_type.short_name())),
                        ),
                ),
        )
        .child(
            // Error message
            div()
                .p_3()
                .rounded_md()
                .bg(rgb(0x5d1a1a))
                .border_1()
                .border_color(rgb(0x991b1b))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0xff4444))
                                .child("⚠ Invalid Page Data"),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0xfca5a5))
                        .mt_2()
                        .child(error_message.clone()),
                ),
        )
        .child(
            // Raw data section for debugging
            div()
                .flex()
                .flex_col()
                .gap_2()
                .mt_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(rgb(0xaaaaaa))
                        .child("Raw Data:"),
                )
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .child(div().text_xs().text_color(rgb(0xcccccc)).child("Cell Count:"))
                        .child(div().text_xs().text_color(rgb(0xcccccc)).child(format!("{}", page.cell_count))),
                )
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .child(div().text_xs().text_color(rgb(0xcccccc)).child("Free Space:"))
                        .child(div().text_xs().text_color(rgb(0xcccccc)).child(format!("{} bytes", page.free_space))),
                )
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .child(div().text_xs().text_color(rgb(0xcccccc)).child("Fragmented:"))
                        .child(div().text_xs().text_color(rgb(0xcccccc)).child(format!("{} bytes", page.fragmented_bytes))),
                )
                .when_some(page.rightmost_pointer, |this, ptr| {
                    this.child(
                        div()
                            .flex()
                            .justify_between()
                            .child(div().text_xs().text_color(rgb(0xcccccc)).child("Right Pointer:"))
                            .child(div().text_xs().text_color(rgb(0xcccccc)).child(format!("→ {}", ptr))),
                    )
                }),
        )
}

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
                        .child(format!("Pages: {}", page_count)),
                )
                .when(is_watching, |this| {
                    this.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(div().size(px(8.0)).rounded_full().bg(rgb(0x4CAF50)))
                            .child(div().text_xs().text_color(rgb(0x4CAF50)).child("Watching")),
                    )
                }),
        )
}

pub fn render_page_grid(pages: &[PageInfo], selected_page: Option<u32>) -> impl IntoElement {
    let mut page_grid = div().flex().flex_wrap().gap_2();

    for page in pages {
        page_grid = page_grid.child(render_page_square(page, selected_page));
    }

    div().flex().flex_1().flex_col().p_4().child(page_grid)
}

pub fn render_page_square(page: &PageInfo, selected_page: Option<u32>) -> impl IntoElement {
    let is_selected = selected_page == Some(page.page_number);

    div()
        .size(px(80.0))
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
        .hover(|this| this.opacity(0.8))
        .id(("page", page.page_number))
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
                .child(if let Some(selected_page_num) = selected_page {
                    if let Some(page) = pages.iter().find(|p| p.page_number == selected_page_num) {
                        render_page_details(
                            page,
                            database_info.map(|info| info.header.actual_page_size()),
                        )
                        .into_any_element()
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
                }),
        )
        .when_some(database_info, |this, info| {
            this.child(
                div()
                    .border_t_1()
                    .border_color(rgb(0x3e3e3e))
                    .p_4()
                    .child(render_database_info(&info.header)),
            )
        })
}

pub fn render_page_details(page: &PageInfo, page_size: Option<usize>) -> impl IntoElement {
    // Validate page data first
    if let Err(validation_error) = validate_page_data(page, page_size) {
        return render_page_details_error(page, &validation_error).into_any_element();
    }
    
    // Safe calculation to prevent integer underflow
    let used_space = page_size
        .map(|size| {
            let size_u16 = size as u16;
            if page.free_space <= size_u16 {
                size_u16 - page.free_space
            } else {
                // Handle invalid data gracefully - free space cannot exceed page size
                0
            }
        })
        .unwrap_or(0);
    
    let total_fragmented = page.fragmented_bytes as u16;
    
    // Safe efficiency calculation with bounds checking
    let efficiency = page_size
        .map(|size| {
            if size == 0 {
                return 0.0;
            }
            
            let size_u16 = size as u16;
            let total_overhead = page.free_space.saturating_add(total_fragmented);
            
            if total_overhead >= size_u16 {
                // Invalid data - overhead cannot exceed page size
                0.0
            } else {
                let usable_space = size_u16 - total_overhead;
                (usable_space as f32 / size as f32) * 100.0
            }
        })
        .unwrap_or(0.0);

    div()
        .flex()
        .flex_col()
        .gap_3()
        .text_color(rgb(0xffffff))
        .child(
            // Page header with color indicator
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .size(px(16.0))
                        .rounded_full()
                        .bg(page.page_type.color()),
                )
                .child(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(format!("Page {}", page.page_number)),
                ),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Page Type:"),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().child(page.page_type.name()))
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0xaaaaaa))
                                .child(format!("({})", page.page_type.short_name())),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Cell Count:"),
                )
                .child(div().child(format!("{}", page.cell_count))),
        )
        .when_some(page_size, |this, size| {
            this.child(
                div()
                    .flex()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Page Size:"),
                    )
                    .child(div().child(format!("{} bytes", size))),
            )
        })
        .child(
            div()
                .flex()
                .justify_between()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Used Space:"),
                )
                .child(div().child(format!("{} bytes", used_space))),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Free Space:"),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().child(format!("{} bytes", page.free_space)))
                        .when_some(page_size, |this, size| {
                            let percentage = if size > 0 {
                                (page.free_space as f32 / size as f32) * 100.0
                            } else {
                                0.0
                            };
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0xaaaaaa))
                                    .child(format!("({:.1}%)", percentage)),
                            )
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Fragmented:"),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().child(format!("{} bytes", page.fragmented_bytes)))
                        .when(page.fragmented_bytes > 0, |this| {
                            this.child(div().text_xs().text_color(rgb(0xff9800)).child("⚠"))
                        }),
                ),
        )
        .when_some(page.rightmost_pointer, |this, ptr| {
            this.child(
                div()
                    .flex()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Right Pointer:"),
                    )
                    .child(div().child(format!("→ {}", ptr))),
            )
        })
        .when_some(page_size, |this, size| {
            this.child(
                div()
                    .flex()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Utilization:"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().child(format!("{:.1}%", page.utilization_percent(size))))
                            .child(
                                div()
                                    .w(px(50.0))
                                    .h(px(8.0))
                                    .bg(rgb(0x333333))
                                    .rounded_sm()
                                    .child(
                                        div()
                                            .h_full()
                                            .rounded_sm()
                                            .bg(if page.utilization_percent(size) > 80.0 {
                                                rgb(0xff4444)
                                            } else if page.utilization_percent(size) > 60.0 {
                                                rgb(0xff9800)
                                            } else {
                                                rgb(0x4CAF50)
                                            })
                                            .w(px((page.utilization_percent(size) / 100.0 * 50.0)
                                                as f32)),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Efficiency:"),
                    )
                    .child(div().child(format!("{:.1}%", efficiency)).text_color(
                        if efficiency > 90.0 {
                            rgb(0x4CAF50)
                        } else if efficiency > 70.0 {
                            rgb(0xff9800)
                        } else {
                            rgb(0xff4444)
                        },
                    )),
            )
        })
        .into_any_element()
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
                .child("Database Info"),
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
                .child(format!("{}", header.database_size_pages)),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xcccccc))
                .flex()
                .justify_between()
                .child("Schema Version:")
                .child(format!("{}", header.schema_format_number)),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xcccccc))
                .flex()
                .justify_between()
                .child("SQLite Version:")
                .child(format!("{}", header.sqlite_version_number)),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xcccccc))
                .flex()
                .justify_between()
                .child("User Version:")
                .child(format!("{}", header.user_version)),
        )
        .when(header.application_id != 0, |this| {
            this.child(
                div()
                    .text_xs()
                    .text_color(rgb(0xcccccc))
                    .flex()
                    .justify_between()
                    .child("App ID:")
                    .child(format!("0x{:08X}", header.application_id)),
            )
        })
}

pub fn render_status_message(message: &str, is_error: bool) -> impl IntoElement {
    let message = message.to_string();
    div()
        .p_3()
        .m_2()
        .rounded_md()
        .bg(if is_error {
            rgb(0x5d1a1a)
        } else {
            rgb(0x1a5d2e)
        })
        .border_1()
        .border_color(if is_error {
            rgb(0x991b1b)
        } else {
            rgb(0x16a34a)
        })
        .child(
            div()
                .text_sm()
                .text_color(if is_error {
                    rgb(0xfca5a5)
                } else {
                    rgb(0x86efac)
                })
                .child(message),
        )
}

pub fn render_loading_indicator() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .p_8()
        .child(div().text_color(rgb(0xaaaaaa)).child("Loading..."))
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
                .child("No database loaded"),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(0x888888))
                .child("Open a SQLite database file to get started"),
        )
        .child(render_open_file_button())
}

pub fn render_open_file_button() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .px_6()
        .py_3()
        .bg(rgb(0x2563eb))
        .hover(|this| this.bg(rgb(0x1d4ed8)))
        .rounded_lg()
        .cursor_pointer()
        .child(
            div()
                .text_sm()
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(rgb(0xffffff))
                .child("Open File"),
        )
        .id("open-file-button")
}

pub fn render_page_statistics(database_info: &DatabaseInfo) -> impl IntoElement {
    let total_pages = database_info.page_count();
    let page_size = database_info.header.actual_page_size();

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
                .child("Database Statistics"),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xcccccc))
                .flex()
                .justify_between()
                .child("Total Pages:")
                .child(format!("{}", total_pages)),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xcccccc))
                .flex()
                .justify_between()
                .child("Page Size:")
                .child(format!("{} bytes", page_size)),
        )
}
