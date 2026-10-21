use std::cell::RefCell;
use std::rc::Rc;

use gloo::events::EventListener;
use gloo::timers::callback::Timeout;
use gloo::utils::document;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Element, HtmlElement, IntersectionObserver, IntersectionObserverInit, Node, Text};

struct TextPiece {
    node: Text,
    chars: Vec<char>,
    at: usize,
}

struct Line {
    el: HtmlElement,
    nodes: Vec<TextPiece>,
    index: usize,
    state: u8,
    visible: bool,
    media: Option<Element>,
    speed: f64,
    blob: bool,
}

struct Blog {
    queue: Vec<Line>,
    current: Option<usize>,
    caret: Option<Element>,
    hint: Option<HtmlElement>,
    timer: Option<Timeout>,
    observer: Option<IntersectionObserver>,
    done: bool,
}

type Shared = Rc<RefCell<Blog>>;

pub fn start() {
    if document().get_element_by_id("bg-water").is_none() {
        return;
    }
    start_water();
    fit_grid();
    let listener = EventListener::new(&gloo::utils::window(), "resize", |_| fit_grid());
    std::mem::forget(listener);

    let hint = document()
        .query_selector(".term-line .hint")
        .ok()
        .flatten()
        .and_then(|node| node.dyn_into::<HtmlElement>().ok());
    let reduced = gloo::utils::window()
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .map(|query| query.matches())
        .unwrap_or(false);

    strip_images();
    let queue = collect();
    if queue.is_empty() {
        return;
    }
    let has_observer = js_sys::Reflect::has(
        &JsValue::from(gloo::utils::window()),
        &JsValue::from_str("IntersectionObserver"),
    )
    .unwrap_or(false);

    let state: Shared = Rc::new(RefCell::new(Blog {
        queue,
        current: None,
        caret: None,
        hint,
        timer: None,
        observer: None,
        done: false,
    }));

    if reduced || !has_observer {
        for line in state.borrow_mut().queue.iter_mut() {
            line.visible = true;
        }
        if reduced {
            finish_all(&state);
        } else {
            pump(&state);
        }
        return;
    }

    observe(&state);
    attach_shortcuts(&state);
}

fn start_water() {
    spawn_local(async move {
        if let Some(ready) = js_sys::Reflect::get(
            &JsValue::from(gloo::utils::window()),
            &JsValue::from_str("waterSimReady"),
        )
        .ok()
        .and_then(|value| value.dyn_into::<js_sys::Promise>().ok())
        {
            let _ = JsFuture::from(ready).await;
        }
        let Some(create) = js_sys::Reflect::get(
            &JsValue::from(gloo::utils::window()),
            &JsValue::from_str("createWaterSim"),
        )
        .ok()
        .and_then(|value| value.dyn_into::<js_sys::Function>().ok())
        else {
            return;
        };
        let Some(mount) = document().get_element_by_id("bg-water") else {
            return;
        };
        let options = js_sys::Object::new();
        let set = |key: &str, value: &JsValue| {
            let _ = js_sys::Reflect::set(&options, &JsValue::from_str(key), value);
        };
        set("mount", &mount);
        set("greeting", &JsValue::from_str(""));
        set("buttons", &js_sys::Array::new());
        set("text", &JsValue::FALSE);
        set("typewriter", &JsValue::FALSE);
        set("interactive", &JsValue::TRUE);
        set("globalInput", &JsValue::TRUE);
        set("easterEggs", &JsValue::FALSE);
        set("pageHooks", &JsValue::FALSE);
        let size = Closure::<dyn Fn() -> JsValue>::new(|| {
            let window = gloo::utils::window();
            let size = js_sys::Object::new();
            let width = window.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
            let height = window.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
            let _ = js_sys::Reflect::set(&size, &JsValue::from_str("width"), &JsValue::from_f64(width));
            let _ = js_sys::Reflect::set(&size, &JsValue::from_str("height"), &JsValue::from_f64(height));
            size.into()
        });
        set("size", size.as_ref());
        size.forget();
        let Ok(water) = create.call1(&JsValue::NULL, &options) else {
            return;
        };
        call(&water, "stir", &[]);
        let _ = js_sys::Reflect::set(
            &JsValue::from(gloo::utils::window()),
            &JsValue::from_str("__blogWater"),
            &water,
        );
        watch_frame(water);
    });
}

fn call(target: &JsValue, name: &str, args: &[JsValue]) -> JsValue {
    let Some(function) = js_sys::Reflect::get(target, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.dyn_into::<js_sys::Function>().ok())
    else {
        return JsValue::UNDEFINED;
    };
    let array = js_sys::Array::new();
    for value in args {
        array.push(value);
    }
    function.apply(target, &array).unwrap_or(JsValue::UNDEFINED)
}

fn watch_frame(water: JsValue) {
    let window = gloo::utils::window();
    let Ok(Some(parent)) = window.parent() else {
        return;
    };
    if JsValue::from(parent.clone()) == JsValue::from(window.clone()) {
        return;
    }
    let Ok(Some(frame)) = window.frame_element() else {
        return;
    };
    let Some(host) = parent.document() else {
        return;
    };

    let pending = Rc::new(RefCell::new(false));
    let debounce: Rc<RefCell<Option<Timeout>>> = Rc::new(RefCell::new(None));

    let sync = {
        let water = water.clone();
        let parent = parent.clone();
        let frame = frame.clone();
        move || {
            let rect = frame.get_bounding_client_rect();
            let height = parent
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let on_screen = rect.bottom() > 0.0 && rect.top() < height;
            let landing = js_sys::Reflect::get(
                &JsValue::from(parent.clone()),
                &JsValue::from_str("__pageWaterSim"),
            )
            .unwrap_or(JsValue::UNDEFINED);
            if on_screen {
                call(&water, "resume", &[]);
                call(&landing, "pause", &[]);
            } else {
                call(&water, "pause", &[]);
                call(&landing, "resume", &[]);
            }
        }
    };

    let schedule = {
        let pending = pending.clone();
        let debounce = debounce.clone();
        let sync = sync.clone();
        Rc::new(move || {
            if *pending.borrow() {
                return;
            }
            *pending.borrow_mut() = true;
            let pending = pending.clone();
            let sync = sync.clone();
            *debounce.borrow_mut() = Some(Timeout::new(120, move || {
                *pending.borrow_mut() = false;
                sync();
            }));
        })
    };

    let listener = {
        let schedule = schedule.clone();
        EventListener::new(&parent, "scroll", move |_| schedule())
    };
    std::mem::forget(listener);
    let listener = {
        let schedule = schedule.clone();
        EventListener::new(&host, "scroll", move |_| schedule())
    };
    std::mem::forget(listener);
    if let Some(root) = host.document_element() {
        let schedule = schedule.clone();
        let listener = EventListener::new(root.as_ref(), "scroll", move |_| schedule());
        std::mem::forget(listener);
    }
    if let Some(body) = host.body() {
        let schedule = schedule.clone();
        let listener = EventListener::new(body.as_ref(), "scroll", move |_| schedule());
        std::mem::forget(listener);
    }
    let listener = {
        let schedule = schedule.clone();
        EventListener::new(&parent, "resize", move |_| schedule())
    };
    std::mem::forget(listener);

    let release = {
        let parent = parent.clone();
        move || {
            let landing = js_sys::Reflect::get(
                &JsValue::from(parent.clone()),
                &JsValue::from_str("__pageWaterSim"),
            )
            .unwrap_or(JsValue::UNDEFINED);
            call(&landing, "resume", &[]);
        }
    };
    let listener = EventListener::new(&window, "pagehide", move |_| release());
    std::mem::forget(listener);

    sync();
}

fn fit_grid() {
    let height = gloo::utils::window()
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(700.0)
        .max(1.0);
    let rows = (height / 14.0).ceil().max(10.0);
    if let Some(root) = document().document_element() {
        if let Some(root) = root.dyn_ref::<HtmlElement>() {
            root.style()
                .set_property("--fs", &format!("{}px", height / rows))
                .ok();
        }
    }
}

fn speed_of(el: &HtmlElement, fallback: f64) -> f64 {
    el.get_attribute("data-speed")
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| !value.is_nan())
        .unwrap_or(fallback)
}

fn make_line(el: HtmlElement) -> Line {
    let tag = el.tag_name();
    let mut line = Line {
        el: el.clone(),
        nodes: Vec::new(),
        index: 0,
        state: 0,
        visible: false,
        media: None,
        speed: speed_of(&el, 0.34),
        blob: false,
    };
    if matches!(tag.as_str(), "IMG" | "HR" | "VIDEO") {
        el.class_list().add_1("tw-hidden").ok();
        line.media = Some(el.into());
        return line;
    }
    if let Ok(walker) = document().create_tree_walker(&el) {
        while let Ok(Some(node)) = walker.next_node() {
            let Some(text) = node.dyn_into::<Text>().ok() else {
                continue;
            };
            let Some(value) = text.node_value() else {
                continue;
            };
            if value.is_empty() {
                continue;
            }
            text.set_node_value(Some(""));
            line.nodes.push(TextPiece {
                node: text,
                chars: value.chars().collect(),
                at: 0,
            });
        }
    }
    line
}

fn collect() -> Vec<Line> {
    let mut queue = Vec::new();
    if let Ok(elements) = document().query_selector_all("[data-type], .post-body > *") {
        for index in 0..elements.length() {
            let Some(node) = elements.get(index) else { continue };
            let Some(el) = node.dyn_into::<HtmlElement>().ok() else {
                continue;
            };
            if el.closest(".post").ok().flatten().is_some() {
                continue;
            }
            if el.offset_parent().is_none() {
                continue;
            }
            let line = make_line(el.clone());
            el.set_attribute("data-tw", &queue.len().to_string()).ok();
            queue.push(line);
        }
    }
    if let Ok(posts) = document().query_selector_all(".post") {
        for index in 0..posts.length() {
            let Some(node) = posts.get(index) else { continue };
            let Some(el) = node.dyn_into::<HtmlElement>().ok() else {
                continue;
            };
            if el.offset_parent().is_none() {
                continue;
            }
            el.class_list().add_1("tw-unloaded").ok();
            el.set_attribute("data-tw", &queue.len().to_string()).ok();
            queue.push(Line {
                el,
                nodes: Vec::new(),
                index: 0,
                state: 0,
                visible: false,
                media: None,
                speed: 0.0,
                blob: true,
            });
        }
    }
    queue
}

fn strip_images() {
    if let Ok(media) = document().query_selector_all(".post-body img, .post-body picture") {
        for index in 0..media.length() {
            if let Some(node) = media.get(index) {
                if let Some(parent) = node.parent_node() {
                    parent.remove_child(&node).ok();
                }
            }
        }
    }
    if let Ok(blocks) = document().query_selector_all(".post-body > *") {
        for index in 0..blocks.length() {
            let Some(node) = blocks.get(index) else { continue };
            let Some(el) = node.dyn_into::<Element>().ok() else {
                continue;
            };
            if el.tag_name() == "HR" {
                continue;
            }
            let empty = el.text_content().map(|text| text.trim().is_empty()).unwrap_or(true);
            let has_media = el.query_selector("img, video, iframe").ok().flatten().is_some();
            if empty && !has_media {
                if let Some(parent) = el.parent_node() {
                    parent.remove_child(&el).ok();
                }
            }
        }
    }
}

fn schedule(state: &Shared, ms: u32, action: fn(&Shared)) {
    let next = state.clone();
    let timer = Timeout::new(ms, move || action(&next));
    state.borrow_mut().timer = Some(timer);
}

fn step(state: &Shared) {
    let mut delay: f64;
    let mut caret_parent: Option<Node> = None;
    let mut caret_next: Option<Node> = None;
    let mut ch = ' ';

    {
        let mut blog = state.borrow_mut();
        let Some(index) = blog.current else {
            return;
        };
        if blog.queue[index].blob {
            blog.queue[index].el.class_list().remove_1("tw-unloaded").ok();
            drop(blog);
            schedule(state, 160, finish_line);
            return;
        }
        let speed = blog.queue[index].speed;
        let burst = if speed < 0.5 { 2 } else { 1 };
        for _ in 0..burst {
            loop {
                let nodes_len = blog.queue[index].nodes.len();
                if blog.queue[index].index >= nodes_len {
                    drop(blog);
                    finish_line(state);
                    return;
                }
                let at = blog.queue[index].nodes[blog.queue[index].index].at;
                let len = blog.queue[index].nodes[blog.queue[index].index].chars.len();
                if at < len {
                    break;
                }
                blog.queue[index].index += 1;
            }
            let piece_index = blog.queue[index].index;
            let piece = &mut blog.queue[index].nodes[piece_index];
            ch = piece.chars[piece.at];
            piece.at += 1;
            while piece.at < piece.chars.len() && piece.chars[piece.at].is_whitespace() {
                piece.at += 1;
            }
            let prefix: String = piece.chars[..piece.at].iter().collect();
            piece.node.set_node_value(Some(&prefix));
            caret_parent = piece.node.parent_node();
            caret_next = piece.node.next_sibling();
        }

        if blog.caret.is_none() {
            if let Ok(span) = document().create_element("span") {
                span.set_class_name("tw-caret");
                span.set_attribute("aria-hidden", "true").ok();
                blog.caret = Some(span);
            }
        }
        if let (Some(caret), Some(parent)) = (blog.caret.as_ref(), caret_parent.as_ref()) {
            parent.insert_before(caret, caret_next.as_ref()).ok();
        }
        delay = 10.0 * speed;
        if ch.is_whitespace() {
            delay *= 0.35;
        } else if ".,;:!?".contains(ch) {
            delay *= 1.6;
        }
    }

    schedule(state, delay.max(1.0) as u32, step);
}

fn finish_line(state: &Shared) {
    {
        let mut blog = state.borrow_mut();
        if let Some(caret) = blog.caret.take() {
            if let Some(parent) = caret.parent_node() {
                parent.remove_child(&caret).ok();
            }
        }
        let Some(index) = blog.current.take() else {
            return;
        };
        if let Some(media) = blog.queue[index].media.clone() {
            media.class_list().remove_1("tw-hidden").ok();
            media.class_list().add_1("tw-shown").ok();
        }
        blog.queue[index].state = 2;
    }
    pump(state);
}

fn flush_current(state: &Shared) {
    {
        let mut blog = state.borrow_mut();
        let Some(index) = blog.current else {
            return;
        };
        if blog.queue[index].blob {
            blog.queue[index].el.class_list().remove_1("tw-unloaded").ok();
        } else {
            let pieces = std::mem::take(&mut blog.queue[index].nodes);
            for piece in pieces {
                let full: String = piece.chars.iter().collect();
                piece.node.set_node_value(Some(&full));
                blog.queue[index].nodes.push(piece);
            }
        }
    }
    finish_line(state);
}

fn pump(state: &Shared) {
    {
        let blog = state.borrow();
        if blog.done || blog.current.is_some() {
            return;
        }
    }
    let next = {
        let blog = state.borrow();
        let mut next = None;
        for (index, line) in blog.queue.iter().enumerate() {
            if line.state == 2 {
                continue;
            }
            if !line.visible {
                return;
            }
            next = Some(index);
            break;
        }
        if next.is_none() {
            if let Some(hint) = &blog.hint {
                hint.style().set_property("display", "none").ok();
            }
            return;
        }
        next
    };
    if let Some(index) = next {
        {
            let mut blog = state.borrow_mut();
            blog.current = Some(index);
            blog.queue[index].state = 1;
        }
        step(state);
    }
}

fn finish_all(state: &Shared) {
    {
        let mut blog = state.borrow_mut();
        if blog.done {
            return;
        }
        blog.done = true;
        blog.timer = None;
        if let Some(caret) = blog.caret.take() {
            if let Some(parent) = caret.parent_node() {
                parent.remove_child(&caret).ok();
            }
        }
        if let Some(observer) = blog.observer.take() {
            observer.disconnect();
        }
        if let Some(hint) = &blog.hint {
            hint.style().set_property("display", "none").ok();
        }
        for line in blog.queue.iter_mut() {
            line.state = 2;
            if line.blob {
                line.el.class_list().remove_1("tw-unloaded").ok();
                continue;
            }
            if let Some(media) = &line.media {
                media.class_list().remove_1("tw-hidden").ok();
                media.class_list().add_1("tw-shown").ok();
                continue;
            }
            for piece in line.nodes.iter_mut() {
                let full: String = piece.chars.iter().collect();
                piece.node.set_node_value(Some(&full));
                piece.at = piece.chars.len();
            }
        }
        blog.current = None;
    }
}

fn observe(state: &Shared) {
    let state_for_callback = state.clone();
    let callback = Closure::<dyn FnMut(js_sys::Array, IntersectionObserver)>::new(
        move |entries: js_sys::Array, observer: IntersectionObserver| {
            for entry in entries.iter() {
                let Ok(entry) = entry.dyn_into::<web_sys::IntersectionObserverEntry>() else {
                    continue;
                };
                if !entry.is_intersecting() {
                    continue;
                }
                let target = entry.target();
                if let Some(index) = target
                    .get_attribute("data-tw")
                    .and_then(|value| value.parse::<usize>().ok())
                {
                    if let Some(line) = state_for_callback.borrow_mut().queue.get_mut(index) {
                        line.visible = true;
                    }
                }
                observer.unobserve(&target);
            }
            pump(&state_for_callback);
        },
    );
    let init = IntersectionObserverInit::new();
    init.set_root_margin("10000px 0px 180px 0px");
    let Ok(observer) = IntersectionObserver::new_with_options(
        callback.as_ref().unchecked_ref(),
        &init,
    ) else {
        return;
    };
    callback.forget();
    for line in state.borrow().queue.iter() {
        observer.observe(line.el.as_ref());
    }
    state.borrow_mut().observer = Some(observer);
}

fn attach_shortcuts(state: &Shared) {
    let finish = state.clone();
    let listener = EventListener::new(&document(), "click", move |_| finish_all(&finish));
    std::mem::forget(listener);

    let finish = state.clone();
    let listener = EventListener::new(&document(), "keydown", move |event| {
        if let Some(event) = event.dyn_ref::<web_sys::KeyboardEvent>() {
            if event.key() == "Escape" {
                finish_all(&finish);
            }
        }
    });
    std::mem::forget(listener);

    let finish = state.clone();
    let listener = EventListener::new(&gloo::utils::window(), "pagehide", move |_| {
        finish_all(&finish)
    });
    std::mem::forget(listener);

    let flush = state.clone();
    let listener = EventListener::new(&gloo::utils::window(), "scroll", move |_| {
        let index = flush.borrow().current;
        let Some(index) = index else { return };
        let el = flush.borrow().queue[index].el.clone();
        if el.get_bounding_client_rect().bottom() < 0.0 {
            flush_current(&flush);
        }
    });
    std::mem::forget(listener);
}
