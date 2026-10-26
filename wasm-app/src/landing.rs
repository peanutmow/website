use std::cell::RefCell;
use std::rc::Rc;

use gloo::events::EventListener;
use gloo::timers::callback::{Interval, Timeout};
use gloo::utils::{document, window};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{HtmlElement, HtmlIFrameElement, HtmlImageElement};

const CELL_PX: f64 = 14.0;
const GAP: f64 = 2.0;
const MOBILE_KEY: &str = "mobile-notice-dismissed";

pub fn start() {
    overlay();
    menu();
    mobile_notice();
    if document().get_element_by_id("site-window").is_some() {
        console_hint();
        crate::site_window::start();
    }
}

fn overlay() {
    let Some(overlay) = element("#ui-overlay") else { return };
    let Some(name) = overlay.query_selector(".name").ok().flatten().and_then(as_html) else {
        return;
    };
    let Some(bar) = element("#button-bar") else { return };
    let Some(bio) = overlay.query_selector(".bio").ok().flatten().and_then(as_html) else {
        return;
    };

    let buttons: Vec<(HtmlElement, String)> = query_all(".btn[data-label]")
        .into_iter()
        .filter_map(|button| {
            let label = button.get_attribute("data-label")?;
            Some((button, label))
        })
        .collect();

    position(&overlay, &name, &bar, &bio, &buttons);

    let pending: Rc<RefCell<Option<Timeout>>> = Rc::new(RefCell::new(None));
    let overlay_resize = overlay.clone();
    let name_resize = name.clone();
    let bar_resize = bar.clone();
    let bio_resize = bio.clone();
    let buttons_resize = buttons.clone();
    let pending_resize = pending.clone();
    let listener = EventListener::new(&window(), "resize", move |_| {
        let overlay = overlay_resize.clone();
        let name = name_resize.clone();
        let bar = bar_resize.clone();
        let bio = bio_resize.clone();
        let buttons = buttons_resize.clone();
        let mut slot = pending_resize.borrow_mut();
        *slot = Some(Timeout::new(80, move || {
            position(&overlay, &name, &bar, &bio, &buttons);
        }));
    });
    std::mem::forget(listener);

    let overlay_load = overlay.clone();
    let name_load = name.clone();
    let bar_load = bar.clone();
    let bio_load = bio.clone();
    let buttons_load = buttons.clone();
    let listener = EventListener::new(&window(), "load", move |_| {
        position(&overlay_load, &name_load, &bar_load, &bio_load, &buttons_load);
    });
    std::mem::forget(listener);

    for ms in [150u32, 600, 1500] {
        let overlay = overlay.clone();
        let name = name.clone();
        let bar = bar.clone();
        let bio = bio.clone();
        let buttons = buttons.clone();
        Timeout::new(ms, move || {
            position(&overlay, &name, &bar, &bio, &buttons);
        })
        .forget();
    }

    let fonts = document().fonts();
    let overlay = overlay.clone();
    let name = name.clone();
    let bar = bar.clone();
    let bio = bio.clone();
    let buttons = buttons.clone();
    spawn_local(async move {
        if let Ok(ready) = fonts.ready() {
            let _ = JsFuture::from(ready).await;
        }
        Timeout::new(100, move || {
            position(&overlay, &name, &bar, &bio, &buttons);
        })
        .forget();
    });
}

fn position(
    overlay: &HtmlElement,
    name: &HtmlElement,
    bar: &HtmlElement,
    bio: &HtmlElement,
    buttons: &[(HtmlElement, String)],
) {
    let window = window();
    let width = window.inner_width().ok().and_then(|value| value.as_f64()).unwrap_or(0.0);
    let height = window.inner_height().ok().and_then(|value| value.as_f64()).unwrap_or(0.0);
    let layout = js_sys::Reflect::get(&window, &JsValue::from_str("__asciiLayout")).unwrap_or(JsValue::UNDEFINED);

    let pre = element("#ascii-water");
    let measured = pre
        .as_ref()
        .map(measure_char_width)
        .unwrap_or(7.5);

    let (char_w, row_h, name_row, btn_row, btn_col, bio_row) = if layout.is_object() {
        (
            get_f64(&layout, "charW").unwrap_or(measured),
            get_f64(&layout, "rowH").unwrap_or_else(|| row_height(height)),
            get_f64(&layout, "nameRow").unwrap_or_else(|| (40.0f64).floor()),
            get_f64(&layout, "btnRow").unwrap_or_else(|| (40.0f64).floor()),
            get_f64(&layout, "btnCol").unwrap_or(0.0),
            get_f64(&layout, "bioRow").unwrap_or_else(|| (40.0f64).floor()),
        )
    } else {
        let rows = (height / CELL_PX).ceil().max(10.0);
        let row_h = height / rows;
        let cols = (width / measured).ceil().max(20.0);
        (
            measured,
            row_h,
            (rows * 0.84).floor(),
            (rows * 0.89).floor(),
            ((cols - 32.0) / 2.0).floor(),
            (rows * 0.94).floor(),
        )
    };

    let name_y = name_row * row_h;
    style(overlay, "top", &px(name_y));
    style(overlay, "height", &px(row_h * 12.0));
    style(name, "position", "absolute");
    style(name, "top", "0");
    style(name, "left", "50%");
    style(name, "transform", "translateX(-50%)");
    style(name, "white-space", "nowrap");

    style(bar, "position", "absolute");
    style(bar, "top", &px((btn_row - name_row) * row_h));
    style(bar, "left", "0");
    style(bar, "width", "100%");
    style(bar, "height", &px(row_h * 2.5));

    let start_x = btn_col * char_w;
    let mut offset = 0.0;
    for (button, label) in buttons {
        let width = label.chars().count() as f64 * char_w;
        style(button, "left", &px(start_x + offset * char_w));
        style(button, "width", &px(width));
        style(button, "height", "100%");
        style(button, "font-size", &px(row_h * 0.7));
        style(button, "letter-spacing", "0");
        offset += label.chars().count() as f64 + GAP;
    }

    style(bio, "position", "absolute");
    style(bio, "top", &px((bio_row - name_row) * row_h));
    style(bio, "left", "50%");
    style(bio, "transform", "translateX(-50%)");
    style(bio, "width", "80%");
    style(bio, "max-width", "600px");
}

fn menu() {
    let Some(panel) = element("#content-panel") else { return };
    let Some(frame) = element("#content-frame")
        .and_then(|element| element.dyn_into::<HtmlIFrameElement>().ok())
    else {
        return;
    };
    let Some(socials) = element("#socials-inline") else { return };

    let mut pages: Vec<(String, String)> = Vec::new();
    for button in query_all(".btn[data-page]") {
        let Some(page) = button.get_attribute("data-page") else { continue };
        if page == "socials" || page == "how" {
            continue;
        }
        let url = button.get_attribute("data-url").unwrap_or_default();
        pages.push((page, url));
    }

    let warmed = Rc::new(RefCell::new(false));
    let socials_loaded = Rc::new(RefCell::new(false));
    let pages = Rc::new(pages);

    for button in query_all(".btn[data-page]") {
        let Some(page) = button.get_attribute("data-page") else { continue };
        let frame = frame.clone();
        let panel = panel.clone();
        let socials = socials.clone();
        let pages = pages.clone();
        let warmed = warmed.clone();
        let socials_loaded = socials_loaded.clone();
        let listener = EventListener::new(button.as_ref(), "click", move |event| {
            event.prevent_default();
            if page == "how" {
                if let Some(function) = js_sys::Reflect::get(&window(), &JsValue::from_str("openInspect"))
                    .ok()
                    .and_then(|value| value.dyn_into::<js_sys::Function>().ok())
                {
                    let _ = function.call0(&JsValue::NULL);
                }
                return;
            }

            if page == "socials" {
                socials.class_list().add_1("visible").ok();
                frame.style().set_property("display", "none").ok();
                frame.set_src("");
                upgrade_socials(&socials, &socials_loaded);
            } else {
                socials.class_list().remove_1("visible").ok();
                frame.style().set_property("display", "").ok();
                if let Some((_, url)) = pages.iter().find(|(name, _)| name == &page) {
                    warm(&page, &warmed);
                    if !url.is_empty() {
                        frame.set_src(url);
                    }
                }
            }

            stir();
            let options = web_sys::ScrollIntoViewOptions::new();
            options.set_behavior(web_sys::ScrollBehavior::Smooth);
            let _ = panel.scroll_into_view_with_scroll_into_view_options(&options);
        });
        std::mem::forget(listener);
    }
}

fn warm(page: &str, warmed: &Rc<RefCell<bool>>) {
    if *warmed.borrow() || page != "projects" {
        return;
    }
    *warmed.borrow_mut() = true;
    for source in ["/assets/tree-preview.avif", "/assets/treebranch-preview.avif"] {
        if let Ok(image) = document().create_element("img") {
            if let Ok(image) = image.dyn_into::<HtmlImageElement>() {
                image.set_src(source);
            }
        }
    }
}

fn upgrade_socials(socials: &HtmlElement, loaded: &Rc<RefCell<bool>>) {
    if *loaded.borrow() {
        return;
    }
    *loaded.borrow_mut() = true;
    let Some(image) = document()
        .create_element("img")
        .ok()
        .and_then(|element| element.dyn_into::<HtmlImageElement>().ok())
    else {
        return;
    };
    let socials = socials.clone();
    let image_for_load = image.clone();
    let listener = EventListener::new(&image, "load", move |_| {
        let source = image_for_load
            .get_attribute("src")
            .unwrap_or_else(|| "/assets/glassblock4k.avif?v=3".to_string());
        socials
            .style()
            .set_property("background-image", &format!("url('{source}')"))
            .ok();
    });
    std::mem::forget(listener);
    image.set_src("/assets/glassblock4k.avif?v=3");
}

fn stir() {
    let sim = js_sys::Reflect::get(&window(), &JsValue::from_str("__pageWaterSim")).unwrap_or(JsValue::UNDEFINED);
    if sim.is_undefined() || sim.is_null() {
        return;
    }
    if let Ok(function) = js_sys::Reflect::get(&sim, &JsValue::from_str("stir")) {
        if let Ok(function) = function.dyn_into::<js_sys::Function>() {
            let _ = function.call0(&sim);
        }
    }
}

fn mobile_notice() {
    let Some(notice) = element("#mobile-notice") else { return };
    let Some(button) = element("#mn-dismiss") else { return };
    if !is_mobile() {
        return;
    }

    if let Some(storage) = window().local_storage().ok().flatten() {
        if storage.get_item(MOBILE_KEY).ok().flatten().is_some() {
            return;
        }
        let storage_clone = storage.clone();
        let notice_clone = notice.clone();
        let listener = EventListener::new(button.as_ref(), "click", move |_| {
            notice_clone.set_attribute("hidden", "").ok();
            storage_clone.set_item(MOBILE_KEY, "1").ok();
        });
        std::mem::forget(listener);
    }

    notice.remove_attribute("hidden").ok();
    let _ = button;
}

fn is_mobile() -> bool {
    if window()
        .match_media("(pointer: coarse)")
        .ok()
        .flatten()
        .map(|query| query.matches())
        .unwrap_or(false)
    {
        return true;
    }
    if js_sys::Reflect::has(&window(), &JsValue::from_str("ontouchstart")).unwrap_or(false)
        && window().inner_width().ok().and_then(|value| value.as_f64()).unwrap_or(0.0) < 1024.0
    {
        return true;
    }
    let agent = window().navigator().user_agent().unwrap_or_default();
    ["Android", "iPhone", "iPad", "iPod", "Mobile", "Windows Phone"]
        .iter()
        .any(|needle| agent.contains(needle))
}

fn console_hint() {
    let hint = Closure::<dyn Fn()>::new(move || {
        web_sys::console::log_1(&JsValue::from_str("Try 42, cowsay, /etc/hosts, /changelog, /man, #about:blank, /dev/null, or How."));
    });
    let _ = js_sys::Reflect::set(&window(), &JsValue::from_str("hint"), hint.as_ref());
    hint.forget();
    web_sys::console::log_1(&JsValue::from_str("LSPK90 — type hint() for a secret."));
}

fn measure_char_width(pre: &HtmlElement) -> f64 {
    let font = pre.style().get_property_value("font-size").unwrap_or_default();
    if font.is_empty() {
        return 7.5;
    }
    let Ok(value) = font.trim_end_matches("px").parse::<f64>() else {
        return 7.5;
    };
    value * 0.6
}

fn row_height(height: f64) -> f64 {
    height / (height / CELL_PX).ceil().max(10.0)
}

fn get_f64(value: &JsValue, key: &str) -> Option<f64> {
    js_sys::Reflect::get(value, &JsValue::from_str(key))
        .ok()
        .and_then(|value| value.as_f64())
}

fn style(element: &HtmlElement, property: &str, value: &str) {
    element.style().set_property(property, value).ok();
}

fn px(value: f64) -> String {
    format!("{value:.4}px")
}

fn element(selector: &str) -> Option<HtmlElement> {
    document()
        .query_selector(selector)
        .ok()
        .flatten()
        .and_then(as_html)
}

fn query_all(selector: &str) -> Vec<HtmlElement> {
    document()
        .query_selector_all(selector)
        .ok()
        .into_iter()
        .flat_map(|list| (0..list.length()).filter_map(move |index| list.item(index)))
        .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
        .filter_map(as_html)
        .collect()
}

fn as_html(element: web_sys::Element) -> Option<HtmlElement> {
    element.dyn_into::<HtmlElement>().ok()
}

#[allow(dead_code)]
fn interval_tick(callback: impl FnMut() + 'static) {
    Interval::new(60, callback).forget();
}
