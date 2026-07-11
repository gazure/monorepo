use dioxus::prelude::*;

/// Prev/next pager. `page` is zero-based; `total_pages` comes from `Page::total_pages`.
#[component]
pub fn Pagination(mut page: Signal<u32>, total_pages: u32, total: i64) -> Element {
    let current = page();
    rsx! {
        div { class: "pagination",
            button {
                disabled: current == 0,
                onclick: move |_| {
                    let p = page();
                    page.set(p.saturating_sub(1));
                },
                "← Prev"
            }
            span { class: "pagination-status",
                "page {current + 1} of {total_pages.max(1)} ({total} rows)"
            }
            button {
                disabled: current + 1 >= total_pages,
                onclick: move |_| {
                    let p = page();
                    page.set(p + 1);
                },
                "Next →"
            }
        }
    }
}
