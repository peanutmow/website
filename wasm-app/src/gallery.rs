use std::cell::RefCell;
use std::rc::Rc;

use gloo::events::EventListener;
use gloo::utils::{document, window};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    Event, HtmlElement, HtmlImageElement, IntersectionObserver, IntersectionObserverEntry,
    KeyboardEvent, MediaQueryList,
};

const SHEET_SIZE: usize = 36;
const COLOUR_BUCKET: i64 = 30;

type Shared = Rc<RefCell<App>>;

struct Photo {
    id: String,
    element: HtmlElement,
    img: Option<HtmlImageElement>,
    alt: String,
    tags: Vec<String>,
    hue: Option<i64>,
    sat: f64,
    meta: JsValue,
    sheet: Option<HtmlElement>,
}

#[derive(Default)]
struct State {
    hue: Option<i64>,
    mono: bool,
    tag: Option<String>,
}

struct App {
    photos: Vec<Photo>,
    state: State,
    last_visible: Vec<usize>,
    lb_list: Vec<usize>,
    lb_index: usize,
    last_focus: Option<HtmlElement>,
    tag_row: HtmlElement,
    status: HtmlElement,
    reset: HtmlElement,
    empty: HtmlElement,
    lightbox: HtmlElement,
    lb_img: HtmlImageElement,
    lb_label: HtmlElement,
    lb_counter: HtmlElement,
    lb_prev: HtmlElement,
    lb_next: HtmlElement,
}

pub fn start() {
    let Some(app) = App::new() else { return };
    let app: Shared = Rc::new(RefCell::new(app));
    bind(&app);
    app.borrow_mut().apply_filters();
    observe_sheets();
    app.borrow_mut().set_title();

    let deep_link = app.borrow().deep_link();
    if let Some(id) = deep_link {
        let app = app.clone();
        gloo::timers::callback::Timeout::new(0, move || {
            app.borrow_mut().open_lightbox(&id);
        })
        .forget();
    }
}

fn bind(app: &Shared) {
    let document = document();
    let window = window();

    for button in query_all(".swatch[data-hue]") {
        if let Some(bucket) = dataset(&button, "hue").and_then(|value| value.parse::<i64>().ok()) {
            let app = app.clone();
            let listener = EventListener::new(button.as_ref(), "click", move |_| {
                app.borrow_mut().toggle_hue(bucket);
            });
            std::mem::forget(listener);
        }
    }

    if let Some(mono) = query_one(".swatch.mono") {
        let app = app.clone();
        let listener = EventListener::new(mono.as_ref(), "click", move |_| {
            app.borrow_mut().toggle_mono();
        });
        std::mem::forget(listener);
    }

    for button in query_all(".tag") {
        let tag = dataset(&button, "tag").unwrap_or_default();
        let app = app.clone();
        let listener = EventListener::new(button.as_ref(), "click", move |_| {
            app.borrow_mut().toggle_tag(&tag);
        });
        std::mem::forget(listener);
    }

    if let Some(reset) = query_one("#reset") {
        let app = app.clone();
        let listener = EventListener::new(reset.as_ref(), "click", move |_| {
            app.borrow_mut().reset_filters();
        });
        std::mem::forget(listener);
    }

    if let Some(previous) = query_one("#lb-prev") {
        let app = app.clone();
        let listener = EventListener::new(previous.as_ref(), "click", move |_| {
            app.borrow_mut().step(-1);
        });
        std::mem::forget(listener);
    }

    if let Some(next) = query_one("#lb-next") {
        let app = app.clone();
        let listener = EventListener::new(next.as_ref(), "click", move |_| {
            app.borrow_mut().step(1);
        });
        std::mem::forget(listener);
    }

    if let Some(close) = query_one("#lb-close") {
        let app = app.clone();
        let listener = EventListener::new(close.as_ref(), "click", move |_| {
            app.borrow_mut().close_lightbox();
        });
        std::mem::forget(listener);
    }

    for frame in query_all(".frame") {
        let Some(id) = dataset(&frame, "id") else { continue };
        let app = app.clone();
        let listener = EventListener::new(frame.as_ref(), "click", move |_| {
            app.borrow_mut().open_lightbox(&id);
        });
        std::mem::forget(listener);
    }

    for image in query_all(".frame img") {
        let Some(frame) = image
            .closest(".frame")
            .ok()
            .flatten()
            .and_then(|node| node.dyn_into::<HtmlElement>().ok())
        else {
            continue;
        };
        let listener = EventListener::new(image.as_ref(), "error", move |_| {
            frame.class_list().add_1("is-broken").ok();
        });
        std::mem::forget(listener);
    }

    for backdrop in query_all("[data-close]") {
        let app = app.clone();
        let listener = EventListener::new(backdrop.as_ref(), "click", move |_| {
            app.borrow_mut().close_lightbox();
        });
        std::mem::forget(listener);
    }

    let app_key = app.clone();
    let listener = EventListener::new(&document, "keydown", move |event| {
        let Some(key) = event.dyn_ref::<KeyboardEvent>().map(KeyboardEvent::key) else {
            return;
        };
        app_key.borrow_mut().on_key(&key, event);
    });
    std::mem::forget(listener);

    let app_hash = app.clone();
    let hash_window = window.clone();
    let listener = EventListener::new(&window, "hashchange", move |_| {
        let hash = hash_window.location().hash().unwrap_or_default();
        let mut app = app_hash.borrow_mut();
        if let Some(id) = parse_hash(&hash) {
            if app.lightbox.hidden() {
                app.open_lightbox(&id);
            }
        } else if !app.lightbox.hidden() {
            app.close_lightbox();
        }
    });
    std::mem::forget(listener);
}

impl App {
    fn new() -> Option<Self> {
        query_one("#archive")?;
        let documents = document();

        let mut photos = Vec::new();
        for element in query_all(".frame") {
            let Some(id) = dataset(&element, "id") else {
                continue;
            };
            let img = element
                .query_selector("img")
                .ok()
                .flatten()
                .and_then(|node| node.dyn_into::<HtmlImageElement>().ok());
            let tags = dataset(&element, "tags")
                .map(|value| parse_string_array(&value))
                .unwrap_or_default();
            let meta = dataset(&element, "meta")
                .map(|value| js_sys::JSON::parse(&value).unwrap_or(JsValue::NULL))
                .unwrap_or(JsValue::NULL);
            let hue = dataset(&element, "hue").and_then(|value| value.parse::<i64>().ok());
            let sat = dataset(&element, "sat")
                .and_then(|value| value.parse::<f64>().ok())
                .unwrap_or(1.0);
            let sheet = element
                .closest(".sheet")
                .ok()
                .flatten()
                .and_then(|node| node.dyn_into::<HtmlElement>().ok());
            let alt = img
                .as_ref()
                .map(|image| image.alt())
                .unwrap_or_else(|| "Photograph".to_string());

            photos.push(Photo {
                id,
                element,
                img,
                alt,
                tags,
                hue,
                sat,
                meta,
                sheet,
            });
        }

        if photos.is_empty() {
            return None;
        }

        let tag_row = query_one("#tags")?;
        let status = query_one("#status")?;
        let reset = query_one("#reset")?;
        let empty = query_one("#empty")?;
        let lightbox = query_one("#lightbox")?;
        let lb_img = documents
            .get_element_by_id("lb-img")?
            .dyn_into::<HtmlImageElement>()
            .ok()?;
        let lb_label = query_one("#lb-label")?;
        let lb_counter = query_one("#lb-counter")?;
        let lb_prev = query_one("#lb-prev")?;
        let lb_next = query_one("#lb-next")?;

        let app = App {
            photos,
            state: State::default(),
            last_visible: Vec::new(),
            lb_list: Vec::new(),
            lb_index: 0,
            last_focus: None,
            tag_row,
            status,
            reset,
            empty,
            lightbox,
            lb_img,
            lb_label,
            lb_counter,
            lb_prev,
            lb_next,
        };

        Some(app)
    }

    fn set_title(&self) {
        let document = document();
        let title = document.title();
        document.set_title(&title.replace("Gallery", &format!("Gallery · {} frames", self.photos.len())));
    }

    fn deep_link(&self) -> Option<String> {
        let hash = window().location().hash().ok()?;
        parse_hash(&hash)
    }

    fn toggle_hue(&mut self, bucket: i64) {
        self.state.hue = if self.state.hue == Some(bucket) {
            None
        } else {
            Some(bucket)
        };
        self.state.mono = false;
        self.apply_filters();
    }

    fn toggle_mono(&mut self) {
        self.state.mono = !self.state.mono;
        if self.state.mono {
            self.state.hue = None;
        }
        self.apply_filters();
    }

    fn toggle_tag(&mut self, tag: &str) {
        if tag.is_empty() {
            self.state.tag = None;
        } else if self.state.tag.as_deref() == Some(tag) {
            self.state.tag = None;
        } else {
            self.state.tag = Some(tag.to_string());
        }
        self.apply_filters();
    }

    fn reset_filters(&mut self) {
        self.state = State::default();
        self.apply_filters();
    }

    fn matches(&self, photo: &Photo) -> bool {
        if self.state.tag.as_deref() == Some("__untagged") {
            if !photo.tags.is_empty() {
                return false;
            }
        } else if let Some(tag) = &self.state.tag {
            if !photo.tags.contains(tag) {
                return false;
            }
        }

        if self.state.mono {
            return photo.sat < 0.15;
        }

        if let Some(hue) = self.state.hue {
            if photo.sat < 0.12 {
                return false;
            }
            return photo.hue.map(|value| value / COLOUR_BUCKET * COLOUR_BUCKET) == Some(hue);
        }

        true
    }

    fn apply_filters(&mut self) {
        let mut visible = Vec::new();
        let mut by_sheet: Vec<(HtmlElement, usize)> = Vec::new();

        for (index, photo) in self.photos.iter().enumerate() {
            let ok = self.matches(photo);
            photo
                .element
                .class_list()
                .toggle_with_force("is-hidden", !ok)
                .ok();
            if ok {
                visible.push(index);
                if let Some(sheet) = &photo.sheet {
                    if let Some(entry) = by_sheet.iter_mut().find(|(item, _)| item.is_same_node(Some(sheet))) {
                        entry.1 += 1;
                    } else {
                        by_sheet.push((sheet.clone(), 1));
                    }
                }
            }
        }

        let mut bits = Vec::new();
        if let Some(hue) = self.state.hue {
            bits.push(hue_name(hue + COLOUR_BUCKET / 2).to_string());
        }
        if self.state.mono {
            bits.push("monochrome".to_string());
        }
        if self.state.tag.as_deref() == Some("__untagged") {
            bits.push("untagged".to_string());
        } else if let Some(tag) = &self.state.tag {
            bits.push(tag.clone());
        }

        for sheet in query_all(".sheet") {
            let shown = by_sheet
                .iter()
                .find(|(item, _)| item.is_same_node(Some(&sheet)))
                .map(|(_, count)| *count)
                .unwrap_or(0);
            let total = dataset(&sheet, "total")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            sheet
                .class_list()
                .toggle_with_force("is-empty", shown == 0)
                .ok();

            if let Some(meta) = sheet
                .query_selector(".sheet-meta")
                .ok()
                .flatten()
                .and_then(|node| node.dyn_into::<HtmlElement>().ok())
            {
                let count = if bits.is_empty() {
                    total.to_string()
                } else {
                    format!("{shown} / {total}")
                };
                let start = dataset(&sheet, "start").unwrap_or_default();
                let end = dataset(&sheet, "end").unwrap_or_default();
                meta.set_text_content(Some(&format!("{count} frames · {start}–{end}")));
            }
        }

        self.last_visible = visible.clone();

        let status = if bits.is_empty() {
            format!("all {} frames", self.photos.len())
        } else {
            let noun = if visible.len() == 1 { "frame" } else { "frames" };
            format!("{} {noun} · {}", visible.len(), bits.join(" · "))
        };
        self.status.set_text_content(Some(&status));
        self.reset.set_hidden(bits.is_empty());
        self.empty.set_hidden(!visible.is_empty());
        self.sync_filter_buttons();
    }

    fn sync_filter_buttons(&self) {
        for button in query_all(".swatch[data-hue]") {
            let pressed = dataset(&button, "hue")
                .and_then(|value| value.parse::<i64>().ok())
                == self.state.hue;
            button
                .set_attribute("aria-pressed", if pressed { "true" } else { "false" })
                .ok();
        }
        if let Some(mono) = query_one(".swatch.mono") {
            mono.set_attribute("aria-pressed", if self.state.mono { "true" } else { "false" })
                .ok();
        }
        let active = self.state.tag.clone().unwrap_or_default();
        for button in self.tag_row.query_selector_all(".tag").ok().into_iter().flat_map(nodes) {
            let tag = dataset(&button, "tag").unwrap_or_default();
            button
                .set_attribute("aria-pressed", if tag == active { "true" } else { "false" })
                .ok();
        }
    }

    fn frame_of(&self, index: usize) -> (usize, usize, usize) {
        (
            index + 1,
            index / SHEET_SIZE + 1,
            self.photos.len().div_ceil(SHEET_SIZE),
        )
    }

    fn open_lightbox(&mut self, id: &str) {
        let Some(index) = self.photos.iter().position(|photo| photo.id == id) else {
            return;
        };

        if !self.last_visible.contains(&index) {
            self.state = State::default();
            self.apply_filters();
        }

        self.lb_list = if self.last_visible.is_empty() {
            vec![index]
        } else {
            self.last_visible.clone()
        };
        self.lb_index = self
            .lb_list
            .iter()
            .position(|item| *item == index)
            .unwrap_or(0);
        self.last_focus = document()
            .active_element()
            .and_then(|element| element.dyn_into::<HtmlElement>().ok());

        self.show_lightbox();
        self.lightbox.set_hidden(false);
        document()
            .body()
            .map(|body| body.class_list().add_1("lb-open").ok());
        if let Some(close) = query_one("#lb-close") {
            let _ = close.focus();
        }
        self.sync_hash(id);
    }

    fn show_lightbox(&mut self) {
        let Some(index) = self.lb_list.get(self.lb_index).copied() else {
            return;
        };
        let Some(photo) = self.photos.get(index) else {
            return;
        };

        let best = json_string(&photo.meta, "best").or_else(|| {
            photo
                .img
                .as_ref()
                .and_then(|img| img.get_attribute("src"))
        });
        let srcset = json_string(&photo.meta, "srcset");

        if let Some(best) = best {
            self.lb_img.set_src(&best);
        }
        if let Some(srcset) = srcset {
            self.lb_img.set_attribute("srcset", &srcset).ok();
            self.lb_img
                .set_attribute("sizes", "(max-width: 900px) 100vw, 70vw")
                .ok();
        } else {
            self.lb_img.remove_attribute("srcset").ok();
        }
        self.lb_img.set_alt(&photo.alt);

        self.render_label(index);
        self.lb_counter.set_text_content(Some(&format!(
            "{:03} / {:03}",
            self.lb_index + 1,
            self.lb_list.len()
        )));
        if self.lb_list.len() < 2 {
            self.lb_prev.set_attribute("disabled", "").ok();
            self.lb_next.set_attribute("disabled", "").ok();
        } else {
            self.lb_prev.remove_attribute("disabled").ok();
            self.lb_next.remove_attribute("disabled").ok();
        }

        self.preload(self.lb_index as i64 - 1);
        self.preload(self.lb_index as i64 + 1);
    }

    fn preload(&self, index: i64) {
        if index < 0 || index as usize >= self.lb_list.len() {
            return;
        }
        let Some(photo) = self.photos.get(self.lb_list[index as usize]) else {
            return;
        };
        if let Some(src) = json_string(&photo.meta, "best") {
            if let Ok(image) = document().create_element("img") {
                if let Ok(image) = image.dyn_into::<HtmlImageElement>() {
                    image.set_src(&src);
                }
            }
        }
    }

    fn render_label(&mut self, index: usize) {
        let Some(photo) = self.photos.get(index) else {
            return;
        };
        self.lb_label.set_inner_html("");
        let (frame, sheet, sheets) = self.frame_of(index);

        let heading = make_element("div");
        heading.set_class_name("lb-frame");
        heading.set_inner_html(&format!(
            "Frame <span class=\"hash\">{frame:03}</span> / {:03}",
            self.photos.len()
        ));
        self.lb_label.append_child(&heading).ok();

        let sheet_label = make_element("div");
        sheet_label.set_class_name("lb-sheet");
        sheet_label.set_text_content(Some(&format!("sheet {sheet:02} / {sheets:02}")));
        self.lb_label.append_child(&sheet_label).ok();

        if !photo.tags.is_empty() {
            let wrap = make_element("div");
            wrap.set_class_name("lb-tags");
            for tag in &photo.tags {
                let span = make_element("span");
                span.set_class_name("lb-tag");
                span.set_text_content(Some(tag));
                wrap.append_child(&span).ok();
            }
            self.lb_label.append_child(&wrap).ok();
        }

        let fields = make_element("div");
        fields.set_class_name("lb-fields");
        let title = json_string(&photo.meta, "title");
        if let Some(title) = title.filter(|value| !value.is_empty() && !is_camera_name(value)) {
            add_field(&fields, "file", &title);
        }
        let width = json_number(&photo.meta, "width");
        let height = json_number(&photo.meta, "height");
        if let (Some(width), Some(height)) = (width, height) {
            add_field(&fields, "size", &format!("{width} × {height}"));
        }
        if let Some(kind) = json_string(&photo.meta, "type") {
            if kind != "photography" {
                add_field(&fields, "type", &kind);
            }
        }
        if let Ok(exif) = js_sys::Reflect::get(&photo.meta, &JsValue::from_str("exif")) {
            for key in ["camera", "lens", "focal", "aperture", "shutter", "iso"] {
                if let Some(value) = json_string(&exif, key) {
                    add_field(&fields, key, &value);
                }
            }
        }
        if fields.child_nodes().length() > 0 {
            self.lb_label.append_child(&fields).ok();
        }

        if let Some(palette) = json_array(&photo.meta, "palette") {
            if !palette.is_empty() {
                let wrap = make_element("div");
                wrap.set_class_name("lb-palette");
                for colour in palette {
                    let swatch = make_element("span");
                    swatch.set_class_name("lb-swatch");
                    swatch.set_attribute("style", &format!("background:{colour}")).ok();
                    swatch.set_attribute("title", &colour).ok();
                    wrap.append_child(&swatch).ok();
                }
                self.lb_label.append_child(&wrap).ok();
            }
        }

        if let Some(caption) = json_string(&photo.meta, "caption").filter(|value| !value.is_empty()) {
            let caption_el = make_element("div");
            caption_el.set_class_name("lb-caption");
            caption_el.set_text_content(Some(&caption));
            self.lb_label.append_child(&caption_el).ok();
        }

        let link = make_element("a");
        link.set_class_name("lb-link");
        let url = json_string(&photo.meta, "url").unwrap_or_else(|| {
            format!("https://www.flickr.com/photos/{}/", photo.id)
        });
        link.set_attribute("href", &url).ok();
        link.set_attribute("target", "_blank").ok();
        link.set_attribute("rel", "noopener").ok();
        link.set_text_content(Some("view on flickr ↗"));
        self.lb_label.append_child(&link).ok();
    }

    fn close_lightbox(&mut self) {
        self.lightbox.set_hidden(true);
        document()
            .body()
            .map(|body| body.class_list().remove_1("lb-open").ok());
        self.lb_img.remove_attribute("src").ok();
        self.lb_img.remove_attribute("srcset").ok();
        self.clear_hash();
        if let Some(focus) = &self.last_focus {
            let _ = focus.focus();
        }
    }

    fn step(&mut self, delta: i64) {
        if self.lb_list.len() < 2 {
            return;
        }
        let length = self.lb_list.len() as i64;
        self.lb_index = ((self.lb_index as i64 + delta + length) % length) as usize;
        self.show_lightbox();
        if let Some(index) = self.lb_list.get(self.lb_index) {
            if let Some(photo) = self.photos.get(*index) {
                self.sync_hash(&photo.id.clone());
            }
        }
    }

    fn on_key(&mut self, key: &str, event: &Event) {
        if self.lightbox.hidden() {
            return;
        }
        if key == "Escape" {
            self.close_lightbox();
            event.prevent_default();
        } else if key == "ArrowLeft" {
            self.step(-1);
            event.prevent_default();
        } else if key == "ArrowRight" {
            self.step(1);
            event.prevent_default();
        } else if key == "Tab" {
            let focusables = self
                .lightbox
                .query_selector_all("button:not([disabled]), a[href]")
                .ok()
                .into_iter()
                .flat_map(nodes)
                .collect::<Vec<_>>();
            let Some(first) = focusables.first() else {
                return;
            };
            let Some(last) = focusables.last() else {
                return;
            };
            let active = document().active_element();
            let active = active.as_ref();
            let shift = event
                .dyn_ref::<KeyboardEvent>()
                .map(|event| event.shift_key())
                .unwrap_or(false);
            if shift && active == Some(first.as_ref()) {
                let _ = last.focus();
                event.prevent_default();
            } else if !shift && active == Some(last.as_ref()) {
                let _ = first.focus();
                event.prevent_default();
            }
        }
    }

    fn sync_hash(&self, id: &str) {
        let location = window().location();
        let path = location.pathname().unwrap_or_default();
        let search = location.search().unwrap_or_default();
        let url = format!("{path}{search}#p/{id}");
        if location.hash().unwrap_or_default() != format!("#p/{id}") {
            if let Ok(history) = window().history() {
                let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&url));
            }
        }
    }

    fn clear_hash(&self) {
        let location = window().location();
        if location.hash().unwrap_or_default().is_empty() {
            return;
        }
        let path = location.pathname().unwrap_or_default();
        let search = location.search().unwrap_or_default();
        if let Ok(history) = window().history() {
            let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&format!("{path}{search}")));
        }
    }
}

fn observe_sheets() {
    let reduce = window()
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .map(|query: MediaQueryList| query.matches())
        .unwrap_or(false);
    let sheets = query_all(".sheet");
    if reduce {
        for sheet in sheets {
            sheet.class_list().add_1("is-in").ok();
        }
        return;
    }

    let callback = Closure::<dyn FnMut(js_sys::Array, IntersectionObserver)>::new(
        move |entries: js_sys::Array, observer: IntersectionObserver| {
            for entry in entries.iter() {
                let Ok(entry) = entry.dyn_into::<IntersectionObserverEntry>() else {
                    continue;
                };
                if !entry.is_intersecting() {
                    continue;
                }
                if let Ok(target) = entry.target().dyn_into::<HtmlElement>() {
                    target.class_list().add_1("is-in").ok();
                }
                observer.unobserve(&entry.target());
            }
        },
    );
    let Ok(observer) = IntersectionObserver::new(callback.as_ref().unchecked_ref()) else {
        return;
    };
    for sheet in sheets {
        observer.observe(&sheet);
    }
    callback.forget();
    std::mem::forget(observer);
}

fn query_all(selector: &str) -> Vec<HtmlElement> {
    document()
        .query_selector_all(selector)
        .ok()
        .into_iter()
        .flat_map(nodes)
        .collect()
}

fn query_one(selector: &str) -> Option<HtmlElement> {
    document()
        .query_selector(selector)
        .ok()
        .flatten()
        .and_then(|node| node.dyn_into::<HtmlElement>().ok())
}

fn nodes(list: web_sys::NodeList) -> Vec<HtmlElement> {
    (0..list.length())
        .filter_map(|index| list.item(index))
        .filter_map(|node| node.dyn_into::<HtmlElement>().ok())
        .collect()
}

fn dataset(element: &HtmlElement, key: &str) -> Option<String> {
    element.dataset().get(key)
}

fn make_element(tag: &str) -> HtmlElement {
    document()
        .create_element(tag)
        .unwrap()
        .dyn_into::<HtmlElement>()
        .unwrap()
}

fn add_field(root: &HtmlElement, name: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    let row = make_element("div");
    let key = make_element("span");
    key.set_text_content(Some(name));
    let val = make_element("b");
    val.set_text_content(Some(value));
    row.append_child(&key).ok();
    row.append_child(&val).ok();
    root.append_child(&row).ok();
}

fn parse_hash(hash: &str) -> Option<String> {
    let value = hash.strip_prefix("#p/")?;
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some(value.to_string())
}

fn parse_string_array(text: &str) -> Vec<String> {
    let Ok(value) = js_sys::JSON::parse(text) else {
        return Vec::new();
    };
    if !js_sys::Array::is_array(&value) {
        return Vec::new();
    }
    js_sys::Array::from(&value)
        .iter()
        .filter_map(|value| value.as_string())
        .collect()
}

fn json_string(value: &JsValue, key: &str) -> Option<String> {
    js_sys::Reflect::get(value, &JsValue::from_str(key))
        .ok()
        .and_then(|value| value.as_string())
}

fn json_number(value: &JsValue, key: &str) -> Option<u32> {
    js_sys::Reflect::get(value, &JsValue::from_str(key))
        .ok()
        .and_then(|value| value.as_f64())
        .map(|value| value as u32)
}

fn json_array(value: &JsValue, key: &str) -> Option<Vec<String>> {
    let array = js_sys::Reflect::get(value, &JsValue::from_str(key)).ok()?;
    if !js_sys::Array::is_array(&array) {
        return None;
    }
    Some(
        js_sys::Array::from(&array)
            .iter()
            .filter_map(|value| value.as_string())
            .collect(),
    )
}

fn is_camera_name(title: &str) -> bool {
    let upper = title.to_uppercase();
    let trimmed = upper.strip_prefix('_').unwrap_or(&upper);
    for prefix in ["DSCN", "DSC", "IMG", "PXL", "GOPR", "DJI", "MVI", "VID"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let rest = rest
                .strip_prefix('-')
                .or_else(|| rest.strip_prefix('_'))
                .unwrap_or(rest);
            if rest.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
                return true;
            }
        }
    }
    (title.len() >= 6 && title.chars().all(|ch| ch.is_ascii_digit()))
        || title.eq_ignore_ascii_case("photo")
}

fn hue_name(hue: i64) -> &'static str {
    const NAMES: &[(i64, &str)] = &[
        (20, "red"),
        (40, "rust"),
        (60, "amber"),
        (80, "olive"),
        (110, "moss"),
        (150, "green"),
        (180, "pine"),
        (200, "teal"),
        (225, "cyan"),
        (250, "steel"),
        (275, "indigo"),
        (305, "violet"),
        (330, "magenta"),
        (350, "rose"),
        (361, "red"),
    ];
    NAMES
        .iter()
        .find(|(limit, _)| hue < *limit)
        .map(|(_, name)| *name)
        .unwrap_or("red")
}

