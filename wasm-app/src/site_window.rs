use std::cell::RefCell;
use std::rc::{Rc, Weak};

use gloo::events::EventListener;
use gloo::render::{request_animation_frame, AnimationFrame};
use gloo::timers::callback::Timeout;
use gloo::utils::{document, window};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlAnchorElement, HtmlElement, HtmlIFrameElement, Url, UrlSearchParams};

const CHARS: &str = " .-:~,^;*=|>+\\/?&$%#@";
const CELL_PX: f64 = 14.0;
const STORAGE_KEY: &str = "alice.siteWindow";

struct Site {
    window: HtmlElement,
    body: HtmlElement,
    card: Option<HtmlElement>,
    ascii: HtmlElement,
    label: Option<HtmlElement>,
    frame: HtmlIFrameElement,
    title: Option<HtmlElement>,
    open: Option<HtmlAnchorElement>,
    rows: usize,
    cols: usize,
    strips: Vec<HtmlElement>,
    raf: Option<AnimationFrame>,
    t: f64,
    revealed: bool,
    open_lock: bool,
    char_width: f64,
    self_ref: Weak<RefCell<Site>>,
}

pub fn start() {
    let Some(win) = element("#site-window") else { return };
    let Some(body) = win
        .query_selector(".sw-body")
        .ok()
        .flatten()
        .and_then(as_html)
    else {
        return;
    };
    let Some(ascii) = element("#site-window-ascii") else { return };
    let Some(frame) = element("#site-window-frame")
        .and_then(|element| element.dyn_into::<HtmlIFrameElement>().ok())
    else {
        return;
    };

    let char_width = measure_char_width();
    let card = win
        .query_selector(".sw-card")
        .ok()
        .flatten()
        .and_then(as_html);
    let label = element("#site-window-label");
    let title = element("#site-window-title");
    let open = element("#site-window-open")
        .and_then(|element| element.dyn_into::<HtmlAnchorElement>().ok());
    let close = element("#site-window-close");

    let site = Rc::new(RefCell::new(Site {
        window: win,
        body,
        card,
        ascii,
        label,
        frame,
        title,
        open,
        rows: 0,
        cols: 0,
        strips: Vec::new(),
        raf: None,
        t: 0.0,
        revealed: false,
        open_lock: false,
        char_width,
        self_ref: Weak::new(),
    }));
    site.borrow_mut().self_ref = Rc::downgrade(&site);

    bind(&site, close);
    maybe_open_pending(&site);
}

fn bind(site: &Rc<RefCell<Site>>, close: Option<HtmlElement>) {
    let body = site.borrow().body.clone();
    let site_move = site.clone();
    let listener = EventListener::new(body.as_ref(), "mousemove", move |_| {
        site_move.borrow_mut().reveal();
    });
    std::mem::forget(listener);

    let body = site.borrow().body.clone();
    let site_touch = site.clone();
    let listener = EventListener::new(body.as_ref(), "touchstart", move |_| {
        site_touch.borrow_mut().reveal();
    });
    std::mem::forget(listener);

    let site_document = site.clone();
    let document_target = document();
    let listener = EventListener::new(&document_target, "mousemove", move |event| {
        let Some(event) = event.dyn_ref::<web_sys::MouseEvent>() else {
            return;
        };
        site_document.borrow_mut().maybe_restore(
            event.client_x() as f64,
            event.client_y() as f64,
        );
    });
    std::mem::forget(listener);

    if let Some(close) = close {
        let site_close = site.clone();
        let listener = EventListener::new(close.as_ref(), "click", move |_| {
            site_close.borrow_mut().close();
        });
        std::mem::forget(listener);
    }

    let window_el = site.borrow().window.clone();
    let site_backdrop = site.clone();
    let listener = EventListener::new(window_el.as_ref(), "click", move |event| {
        let Some(target) = event.target().and_then(|target| target.dyn_into::<web_sys::Element>().ok()) else {
            return;
        };
        if target.closest(".sw-card").ok().flatten().is_none() {
            site_backdrop.borrow_mut().close();
        }
    });
    std::mem::forget(listener);

    let site_key = site.clone();
    let listener = EventListener::new(&document_target, "keydown", move |event| {
        let Some(event) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
            return;
        };
        if event.key() == "Escape" && !site_key.borrow().window.hidden() {
            site_key.borrow_mut().close();
        }
    });
    std::mem::forget(listener);

    for link in query_all(".si-list[data-embed] a") {
        let Some(anchor) = link.dyn_into::<HtmlAnchorElement>().ok() else {
            continue;
        };
        let url = anchor.href();
        let name = anchor
            .get_attribute("data-name")
            .unwrap_or_else(|| host_of(&url));
        let site_link = site.clone();
        let listener = EventListener::new(anchor.as_ref(), "click", move |event| {
            event.prevent_default();
            let _ = window().scroll_to_with_x_and_y(0.0, 0.0);
            let site_link = site_link.clone();
            let url = url.clone();
            let name = name.clone();
            Timeout::new(450, move || {
                site_link.borrow_mut().open(&url, &name);
            })
            .forget();
        });
        std::mem::forget(listener);
    }
}

impl Site {
    fn measure(&mut self) -> f64 {
        self.char_width
    }

    fn build_grid(&mut self) {
        let height = self.body.client_height() as f64;
        let width = self.body.client_width() as f64;
        self.rows = ((height / CELL_PX).floor() as i64).max(6) as usize;
        self.cols = ((width / self.measure()).floor() as i64).max(10) as usize;
        self.ascii.set_inner_html("");
        self.strips.clear();
        for _ in 0..self.rows {
            let row = document().create_element("div").unwrap();
            let row = row.dyn_into::<HtmlElement>().unwrap();
            row.set_class_name("sw-row");
            row.style()
                .set_property("height", &format!("{CELL_PX}px"))
                .ok();
            row.style()
                .set_property("font-size", &format!("{CELL_PX}px"))
                .ok();
            self.ascii.append_child(&row).ok();
            self.strips.push(row);
        }
    }

    fn render_frame(&mut self) {
        self.t += 0.03;
        for (j, strip) in self.strips.iter().enumerate() {
            let mut line = String::with_capacity(self.cols);
            for i in 0..self.cols {
                let value = ((i as f64 / self.cols as f64) * std::f64::consts::PI * 3.0 + self.t).sin() * 0.35
                    + ((j as f64 / self.rows as f64) * std::f64::consts::PI * 2.4 - self.t * 0.45).sin() * 0.25
                    + (((i + j) as f64) * 0.16 + self.t * 0.9).sin() * 0.2
                    + ((i as f64) * 0.35 - (j as f64) * 0.2 + self.t * 0.6).sin() * 0.1;
                let d = (value * 0.5 + 0.5 + js_sys::Math::random() * 0.05).clamp(0.0, 1.0);
                let index = ((d * (CHARS.len() - 1) as f64).round() as usize).min(CHARS.len() - 1);
                line.push(CHARS.as_bytes()[index] as char);
            }
            strip.set_text_content(Some(&line));
        }
    }

    fn start_water(&mut self) {
        if self.raf.is_some() {
            return;
        }
        let Some(state) = self.self_ref.upgrade() else {
            return;
        };
        self.raf = Some(schedule(state));
    }

    fn stop_water(&mut self) {
        self.raf = None;
    }

    fn reveal(&mut self) {
        if self.revealed || self.open_lock || self.strips.is_empty() {
            return;
        }
        self.revealed = true;
        self.body.class_list().add_1("sw-revealed").ok();
        self.stop_water();
    }

    fn restore(&mut self) {
        if !self.revealed {
            return;
        }
        self.revealed = false;
        for strip in &self.strips {
            strip.style().set_property("transition-delay", "0ms").ok();
            strip.style().set_property("transform", "").ok();
            strip.style().set_property("opacity", "").ok();
        }
        self.body.class_list().remove_1("sw-revealed").ok();
    }

    fn maybe_restore(&mut self, x: f64, y: f64) {
        if !self.revealed {
            return;
        }
        let Some(card) = &self.card else { return };
        let rect = card.get_bounding_client_rect();
        if x < rect.left() || x > rect.right() || y < rect.top() || y > rect.bottom() {
            self.restore();
        }
    }

    fn reset_wave(&mut self) {
        self.revealed = false;
        self.body.class_list().remove_1("sw-revealed").ok();
        for strip in &self.strips {
            strip.style().set_property("transition-delay", "0ms").ok();
            strip.style().set_property("opacity", "").ok();
            strip.style().set_property("transform", "").ok();
        }
    }

    fn open(&mut self, url: &str, name: &str) {
        let label = name
            .trim_start_matches("www.")
            .to_uppercase();
        if let Some(label_el) = &self.label {
            label_el.set_text_content(Some(&label));
        }
        if let Some(title) = &self.title {
            title.set_text_content(Some(name));
        }
        if let Some(open) = &self.open {
            open.set_href(url);
        }
        self.frame.set_src(url);
        self.window.set_hidden(false);
        if let Some(root) = document().document_element() {
            if let Ok(root) = root.dyn_into::<HtmlElement>() {
                root.style().set_property("overflow", "hidden").ok();
            }
        }
        self.build_grid();
        self.reset_wave();
        self.start_water();
        self.open_lock = true;
        let self_ref = self.self_ref.clone();
        Timeout::new(600, move || {
            if let Some(state) = self_ref.upgrade() {
                state.borrow_mut().open_lock = false;
            }
        })
        .forget();
    }

    fn close(&mut self) {
        self.window.set_hidden(true);
        self.frame.set_src("about:blank");
        self.stop_water();
        self.revealed = false;
        self.body.class_list().remove_1("sw-revealed").ok();
        if let Some(root) = document().document_element() {
            if let Ok(root) = root.dyn_into::<HtmlElement>() {
                root.style().set_property("overflow", "").ok();
            }
        }
    }
}

fn schedule(state: Rc<RefCell<Site>>) -> AnimationFrame {
    request_animation_frame(move |_| {
        let mut site = state.borrow_mut();
        site.render_frame();
        let next = schedule(state.clone());
        site.raf = Some(next);
    })
}

fn maybe_open_pending(site: &Rc<RefCell<Site>>) {
    let search = window().location().search().unwrap_or_default();
    let mut url = None;
    let mut name = None;
    if let Ok(params) = UrlSearchParams::new_with_str(&search) {
        if let Some(value) = params.get("site") {
            url = Some(value);
            name = params.get("name");
            if let Some(storage) = window().session_storage().ok().flatten() {
                storage.remove_item(STORAGE_KEY).ok();
            }
        }
    }
    if url.is_none() {
        if let Some(storage) = window().session_storage().ok().flatten() {
            if let Ok(Some(stored)) = storage.get_item(STORAGE_KEY) {
                url = Some(stored);
                storage.remove_item(STORAGE_KEY).ok();
            }
        }
    }
    let Some(url) = url else { return };
    let name = name.unwrap_or_else(|| host_of(&url));
    let site = site.clone();
    Timeout::new(700, move || {
        site.borrow_mut().open(&url, &name);
        if window().location().search().unwrap_or_default().contains("site=") {
            let location = window().location();
            let path = location.pathname().unwrap_or_default();
            if let Ok(history) = window().history() {
                let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&path));
            }
        }
    })
    .forget();
}

fn host_of(value: &str) -> String {
    Url::new_with_base(value, &window().location().href().unwrap_or_default())
        .ok()
        .and_then(|url| {
            let host = url.hostname();
            if host.is_empty() { None } else { Some(host) }
        })
        .unwrap_or_else(|| value.to_string())
}

fn measure_char_width() -> f64 {
    let probe = document().create_element("span").unwrap();
    let probe = probe.dyn_into::<HtmlElement>().unwrap();
    probe
        .style()
        .set_css_text("position:fixed;left:-9999px;top:0;visibility:hidden;white-space:pre;font-family:\"Courier New\",\"Share Tech Mono\",monospace;line-height:1;letter-spacing:0;font-size:14px;");
    document().body().unwrap().append_child(&probe).ok();
    probe.set_text_content(Some(&"M".repeat(200)));
    let width = probe.offset_width() as f64 / 200.0;
    probe.remove();
    if width > 0.0 { width } else { 8.0 }
}

fn element(selector: &str) -> Option<HtmlElement> {
    document()
        .query_selector(selector)
        .ok()
        .flatten()
        .and_then(as_html)
}

fn query_all(selector: &str) -> Vec<web_sys::Element> {
    document()
        .query_selector_all(selector)
        .ok()
        .into_iter()
        .flat_map(|list| (0..list.length()).filter_map(move |index| list.item(index)))
        .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
        .collect()
}

fn as_html(element: web_sys::Element) -> Option<HtmlElement> {
    element.dyn_into::<HtmlElement>().ok()
}
