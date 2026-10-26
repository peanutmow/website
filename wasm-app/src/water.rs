use std::cell::RefCell;
use std::rc::Rc;

use gloo::events::EventListener;
use gloo::render::{request_animation_frame, AnimationFrame};
use gloo::timers::callback::{Interval, Timeout};
use gloo::utils::{document, window};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use wasm_water_sim::Sim;
use web_sys::{
    HtmlCanvasElement, HtmlElement, MouseEvent, Text, TouchEvent, TouchList,
};

const CELL_PX: f64 = 14.0;
const FRAME_MS: f64 = 1000.0 / 60.0;
const MAX_CATCHUP: f64 = 3.0;

struct Options {
    greeting: String,
    buttons: Vec<String>,
    mode: String,
    text: bool,
    typewriter: bool,
    page_hooks: bool,
    easter_eggs: bool,
    interactive: bool,
    global_input: bool,
    button_bar: Option<HtmlElement>,
    input_el: Option<HtmlElement>,
    size: Option<js_sys::Function>,
    on_layout: Option<js_sys::Function>,
}

struct Water {
    sim: Sim,
    mount: HtmlElement,
    opts: Options,
    reduce_motion: bool,
    cols: u32,
    rows: u32,
    char_width: f64,
    char_span: Option<HtmlElement>,
    text_node: Option<Text>,
    raf: Option<AnimationFrame>,
    last_timestamp: f64,
    pending_ms: f64,
    typewriter: Option<Interval>,
    revealed: usize,
    last_layout: String,
    listeners: Vec<EventListener>,
    disposed: bool,
}

type Shared = Rc<RefCell<Water>>;

pub fn start() {
    install_globals();
    boot_landing();
}

fn install_globals() {
    let window = window();
    let create = Closure::<dyn Fn(JsValue) -> JsValue>::new(|options: JsValue| create(&options));
    let _ = js_sys::Reflect::set(
        &window,
        &JsValue::from_str("createWaterSim"),
        create.as_ref(),
    );
    create.forget();
    let ready = js_sys::Promise::resolve(&JsValue::UNDEFINED);
    let _ = js_sys::Reflect::set(&window, &JsValue::from_str("waterSimReady"), &ready);
}

fn boot_landing() {
    let Some(mount) = document()
        .get_element_by_id("ascii-water")
        .and_then(|node| node.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let greeting = mount
        .dataset()
        .get("greeting")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Hi, I'm Alice".to_string());
    let buttons = mount
        .dataset()
        .get("buttons")
        .map(|value| {
            value
                .split('|')
                .filter(|part| !part.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            vec![
                "Socials".to_string(),
                "Gallery".to_string(),
                "Blog".to_string(),
                "Projects".to_string(),
            ]
        });
    let options = js_sys::Object::new();
    set(&options, "mount", mount);
    set(&options, "greeting", greeting);
    let button_array = js_sys::Array::new();
    for button in &buttons {
        button_array.push(&JsValue::from_str(button));
    }
    set(&options, "buttons", button_array);
    set(&options, "pageHooks", true);
    set(&options, "globalInput", true);
    set(&options, "easterEggs", true);
    if let Some(bar) = document().get_element_by_id("button-bar") {
        set(&options, "buttonBar", bar);
    }

    let on_layout = Closure::<dyn Fn(JsValue)>::new(|layout: JsValue| {
        let _ = js_sys::Reflect::set(
            &window(),
            &JsValue::from_str("__asciiLayout"),
            &layout,
        );
    });
    set(&options, "onLayout", on_layout.as_ref());
    on_layout.forget();

    let size = Closure::<dyn Fn() -> JsValue>::new(|| {
        let window = window();
        let size = js_sys::Object::new();
        let width = window
            .inner_width()
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0);
        let height = window
            .inner_height()
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0);
        set(&size, "width", width);
        set(&size, "height", height);
        size.into()
    });
    set(&options, "size", size.as_ref());
    size.forget();

    let api = create(&options.into());
    let _ = js_sys::Reflect::set(
        &window(),
        &JsValue::from_str("__pageWaterSim"),
        &api,
    );
}

fn create(options: &JsValue) -> JsValue {
    let mount = option(&options, "mount")
        .and_then(|value| value.dyn_into::<HtmlElement>().ok())
        .or_else(|| {
            document()
                .get_element_by_id("ascii-water")
                .and_then(|node| node.dyn_into::<HtmlElement>().ok())
        });
    let Some(mount) = mount else {
        web_sys::console::error_1(&JsValue::from_str("water: no mount element"));
        return JsValue::NULL;
    };

    let greeting = option(&options, "greeting")
        .and_then(|value| value.as_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Hi, I'm Alice".to_string());
    let buttons = option(&options, "buttons")
        .and_then(|value| value.dyn_into::<js_sys::Array>().ok())
        .map(|array| {
            array
                .iter()
                .filter_map(|entry| entry.as_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            vec![
                "Socials".to_string(),
                "Gallery".to_string(),
                "Blog".to_string(),
                "Projects".to_string(),
            ]
        });
    let opts = Options {
        greeting,
        buttons,
        mode: option(&options, "mode")
            .and_then(|value| value.as_string())
            .unwrap_or_else(|| "ascii".to_string()),
        text: option(&options, "text")
            .and_then(|value| value.as_bool())
            .unwrap_or(true),
        typewriter: option(&options, "typewriter")
            .and_then(|value| value.as_bool())
            .unwrap_or(true),
        page_hooks: option(&options, "pageHooks")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        easter_eggs: option(&options, "easterEggs")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        interactive: option(&options, "interactive")
            .and_then(|value| value.as_bool())
            .unwrap_or(true),
        global_input: option(&options, "globalInput")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        button_bar: option(&options, "buttonBar")
            .and_then(|value| value.dyn_into::<HtmlElement>().ok()),
        input_el: option(&options, "inputEl")
            .and_then(|value| value.dyn_into::<HtmlElement>().ok()),
        size: option(&options, "size")
            .and_then(|value| value.dyn_into::<js_sys::Function>().ok()),
        on_layout: option(&options, "onLayout")
            .and_then(|value| value.dyn_into::<js_sys::Function>().ok()),
    };

    let reduce_motion = window()
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .map(|query| query.matches())
        .unwrap_or(false);
    let seed = js_sys::Reflect::get(&window(), &JsValue::from_str("__SIM_SEED"))
        .ok()
        .and_then(|value| value.as_f64())
        .filter(|value| value.is_finite())
        .map(|value| value as u32)
        .unwrap_or_else(|| (js_sys::Math::random() * 4_294_967_296.0) as u32);

    let shared: Shared = Rc::new(RefCell::new(Water {
        sim: Sim::new(1, 1, seed),
        mount,
        opts,
        reduce_motion,
        cols: 1,
        rows: 1,
        char_width: 8.0,
        char_span: None,
        text_node: None,
        raf: None,
        last_timestamp: 0.0,
        pending_ms: 0.0,
        typewriter: None,
        revealed: 0,
        last_layout: String::new(),
        listeners: Vec::new(),
        disposed: false,
    }));

    on_resize(&shared);
    {
        let mut water = shared.borrow_mut();
        let greeting = water.opts.greeting.clone();
        let buttons = water.opts.buttons.clone();
        let mode = water.opts.mode.clone();
        let text = water.opts.text;
        let typewriter = water.opts.typewriter;
        let page_hooks = water.opts.page_hooks;
        let easter_eggs = water.opts.easter_eggs;
        let reduce_motion = water.reduce_motion;
        water.sim.configure(
            &greeting,
            &buttons,
            &mode,
            text,
            typewriter,
            page_hooks,
            easter_eggs,
            reduce_motion,
        );
    }
    on_resize(&shared);
    trigger_hash_egg(&shared);
    if shared.borrow().opts.typewriter {
        shared.borrow_mut().sim.typewriter_start();
        start_typewriter(&shared);
    } else {
        let mut water = shared.borrow_mut();
        water.sim.typewriter_done();
        water.revealed = water.opts.greeting.chars().count();
    }
    render(&shared);
    install_listeners(&shared);
    resume(&shared);

    build_api(shared)
}

fn option(options: &JsValue, key: &str) -> Option<JsValue> {
    if !options.is_object() {
        return None;
    }
    js_sys::Reflect::get(options, &JsValue::from_str(key))
        .ok()
        .filter(|value| !value.is_undefined() && !value.is_null())
}

fn set(target: &js_sys::Object, key: &str, value: impl Into<JsValue>) {
    let _ = js_sys::Reflect::set(target, &JsValue::from_str(key), &value.into());
}

fn on_resize(state: &Shared) {
    let size = {
        let water = state.borrow();
        water.size()
    };
    let Some((width, height)) = size else {
        return;
    };
    let mut water = state.borrow_mut();
    water.mount.style().set_property("width", &format!("{width}px")).ok();
    water.mount.style().set_property("height", &format!("{height}px")).ok();
    water.allocate(width, height);
    water.update_pre_font_size(height);
}

impl Water {
    fn size(&self) -> Option<(f64, f64)> {
        if let Some(size) = &self.opts.size {
            if let Ok(value) = size.call0(&JsValue::NULL) {
                let width = option(&value, "width")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0);
                let height = option(&value, "height")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0);
                if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
                    return Some((width, height));
                }
            }
        }
        let width = window()
            .inner_width()
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0);
        let height = window()
            .inner_height()
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0);
        if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
            Some((width, height))
        } else {
            None
        }
    }

    fn font_size(&self) -> f64 {
        self.mount
            .style()
            .get_property_value("font-size")
            .ok()
            .and_then(|value| value.trim_end_matches("px").parse::<f64>().ok())
            .unwrap_or(CELL_PX)
    }

    fn measure_char_width(&mut self) -> f64 {
        let Some(size) = self
            .mount
            .style()
            .get_property_value("font-size")
            .ok()
            .filter(|value| !value.is_empty())
        else {
            return 8.0;
        };
        if self.char_span.is_none() {
            let Ok(span) = document().create_element("span") else {
                return size.trim_end_matches("px").parse::<f64>().unwrap_or(8.0) * 0.54;
            };
            span.set_attribute(
                "style",
                "position:fixed;left:0;top:0;visibility:hidden;pointer-events:none;white-space:pre;font-family:'Courier New',monospace;line-height:1;letter-spacing:0;",
            )
            .ok();
            if let Some(body) = document().body() {
                body.append_child(&span).ok();
            }
            if let Ok(span) = span.dyn_into::<HtmlElement>() {
                self.char_span = Some(span);
            }
        }
        if let Some(span) = &self.char_span {
            span.style().set_property("font-size", &size).ok();
            span.set_text_content(Some(&"M".repeat(200)));
            let width = span.offset_width() as f64 / 200.0;
            if width.is_finite() && width > 0.0 {
                self.char_width = width;
                return width;
            }
        }
        let fallback = size.trim_end_matches("px").parse::<f64>().unwrap_or(8.0) * 0.54;
        self.char_width = fallback;
        fallback
    }

    fn allocate(&mut self, width: f64, height: f64) {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return;
        }
        let new_rows = (height / CELL_PX).ceil().max(10.0) as u32;
        let font = height / new_rows as f64;
        if !font.is_finite() || font <= 0.0 {
            return;
        }
        self.mount
            .style()
            .set_property("font-size", &format!("{font}px"))
            .ok();
        self.mount
            .style()
            .set_property("line-height", &format!("{font}px"))
            .ok();
        let cw = self.measure_char_width();
        if !cw.is_finite() || cw <= 0.0 {
            return;
        }
        let new_cols = (width / cw).ceil().max(20.0) as u32;
        if new_cols != self.cols || new_rows != self.rows {
            self.cols = new_cols;
            self.rows = new_rows;
            self.sim.resize(new_cols, new_rows);
        }
    }

    fn update_pre_font_size(&mut self, height: f64) {
        if self.rows == 0 {
            return;
        }
        let font = height / self.rows as f64;
        self.mount
            .style()
            .set_property("font-size", &format!("{font}px"))
            .ok();
        self.mount
            .style()
            .set_property("line-height", &format!("{font}px"))
            .ok();
        self.measure_char_width();
    }

    fn char_px(&self) -> f64 {
        if self.char_width > 0.0 {
            self.char_width
        } else {
            let fallback = self.font_size() * 0.54;
            if fallback > 0.0 {
                fallback
            } else {
                8.0
            }
        }
    }

    fn local_point(&self, client_x: f64, client_y: f64) -> (f64, f64) {
        let rect = self.mount.get_bounding_client_rect();
        (client_x - rect.left(), client_y - rect.top())
    }

    fn grid_point(&self, client_x: f64, client_y: f64) -> (f64, f64) {
        let (x, y) = self.local_point(client_x, client_y);
        (x / self.char_px(), y / self.font_size())
    }
}

fn render(state: &Shared) {
    let layout = {
        let mut water = state.borrow_mut();
        let hidden = if water.opts.page_hooks {
            let scrolled = window()
                .scroll_y()
                .unwrap_or(0.0)
                .max(
                    document()
                        .document_element()
                        .map(|element| element.scroll_top() as f64)
                        .unwrap_or(0.0),
                )
                .max(
                    document()
                        .body()
                        .map(|body| body.scroll_top() as f64)
                        .unwrap_or(0.0),
                );
            let hidden = scrolled > window().inner_height().ok().and_then(|value| value.as_f64()).unwrap_or(0.0) * 0.4;
            if let Some(body) = document().body() {
                body.class_list().toggle_with_force("hide-ui-text", hidden).ok();
            }
            hidden
        } else {
            false
        };
        let text = water.sim.render(hidden);
        let mode = water.sim.mode().to_string();
        if water.mount.get_attribute("data-mode").as_deref() != Some(mode.as_str()) {
            water.mount.set_attribute("data-mode", &mode).ok();
        }
        let same_parent = water
            .text_node
            .as_ref()
            .and_then(|node| node.parent_node())
            .map(|parent| parent.is_same_node(Some(water.mount.as_ref())))
            .unwrap_or(false);
        if same_parent {
            if let Some(node) = &water.text_node {
                node.set_data(&text);
            }
        } else {
            water.mount.set_text_content(Some(&text));
            water.text_node = water
                .mount
                .first_child()
                .and_then(|node| node.dyn_into::<Text>().ok());
        }
        let layout = water.sim.layout_json();
        if layout != water.last_layout {
            water.last_layout = layout.clone();
            water.opts.on_layout.clone().map(|callback| (callback, layout))
        } else {
            None
        }
    };
    if let Some((callback, layout)) = layout {
        if let Ok(parsed) = js_sys::JSON::parse(&layout) {
            callback.call1(&JsValue::NULL, &parsed).ok();
        }
    }
}

fn start_typewriter(state: &Shared) {
    let total = {
        let mut water = state.borrow_mut();
        water.revealed = 0;
        water.typewriter = None;
        water.opts.greeting.chars().count()
    };
    let shared = state.clone();
    let interval = Interval::new(60, move || {
        let done = {
            let mut water = shared.borrow_mut();
            water.revealed += 1;
            water.sim.typewriter_tick();
            water.revealed >= total
        };
        if done {
            shared.borrow_mut().typewriter = None;
        }
    });
    state.borrow_mut().typewriter = Some(interval);
}

fn trigger_hash_egg(state: &Shared) {
    let hash = window().location().hash().unwrap_or_default();
    if hash != "#about:blank" {
        return;
    }
    state.borrow_mut().sim.trigger_about_blank();
    replace_hash();
}

fn replace_hash() {
    let location = window().location();
    let path = format!(
        "{}{}",
        location.pathname().unwrap_or_default(),
        location.search().unwrap_or_default()
    );
    if let Ok(history) = window().history() {
        history
            .replace_state_with_url(&JsValue::NULL, "", Some(&path))
            .ok();
    }
}

fn install_listeners(state: &Shared) {
    let mut listeners = Vec::new();

    let resize_state = state.clone();
    listeners.push(EventListener::new(&window(), "resize", move |_| {
        on_resize(&resize_state)
    }));

    let interactive = state.borrow().opts.interactive;
    if interactive {
        let target: web_sys::EventTarget = {
            let water = state.borrow();
            if water.opts.global_input {
                document().into()
            } else if let Some(input) = &water.opts.input_el {
                input.clone().into()
            } else {
                water.mount.clone().into()
            }
        };

        let move_state = state.clone();
        listeners.push(EventListener::new(&target, "mousemove", move |event| {
            let Some(event) = event.dyn_ref::<MouseEvent>() else {
                return;
            };
            let point = {
                let water = move_state.borrow();
                water.grid_point(event.client_x() as f64, event.client_y() as f64)
            };
            move_state
                .borrow_mut()
                .sim
                .pointer_move(point.0, point.1);
        }));

        let down_state = state.clone();
        listeners.push(EventListener::new(&target, "mousedown", move |event| {
            let Some(event) = event.dyn_ref::<MouseEvent>() else {
                return;
            };
            let point = {
                let water = down_state.borrow();
                water.grid_point(event.client_x() as f64, event.client_y() as f64)
            };
            down_state
                .borrow_mut()
                .sim
                .pointer_down(Some(point.0), Some(point.1));
        }));

        listeners.push(EventListener::new(&target, "mouseup", |_| {}));

        let leave_state = state.clone();
        listeners.push(EventListener::new(&target, "mouseleave", move |_| {
            leave_state.borrow_mut().sim.pointer_leave();
        }));

        let touch_move_state = state.clone();
        listeners.push(EventListener::new(&target, "touchmove", move |event| {
            let Some(event) = event.dyn_ref::<TouchEvent>() else {
                return;
            };
            let Some(touch) = touch_at(&event.touches(), 0) else {
                return;
            };
            let point = {
                let water = touch_move_state.borrow();
                water.grid_point(touch.0, touch.1)
            };
            touch_move_state
                .borrow_mut()
                .sim
                .pointer_move(point.0, point.1);
        }));

        let touch_start_state = state.clone();
        listeners.push(EventListener::new(&target, "touchstart", move |event| {
            let Some(event) = event.dyn_ref::<TouchEvent>() else {
                return;
            };
            let touch = touch_at(&event.touches(), 0)
                .or_else(|| touch_at(&event.changed_touches(), 0));
            let Some(touch) = touch else {
                return;
            };
            let point = {
                let water = touch_start_state.borrow();
                water.grid_point(touch.0, touch.1)
            };
            touch_start_state
                .borrow_mut()
                .sim
                .pointer_down(Some(point.0), Some(point.1));
        }));
    }

    if let Some(bar) = state.borrow().opts.button_bar.clone() {
        if let Ok(buttons) = bar.query_selector_all(".btn") {
            for index in 0..buttons.length() {
                let Some(node) = buttons.get(index) else { continue };
                let Some(button) = node.dyn_into::<HtmlElement>().ok() else {
                    continue;
                };
                let enter_state = state.clone();
                let enter_button = button.clone();
                listeners.push(EventListener::new(button.as_ref(), "mouseenter", move |_| {
                    let rect = enter_button.get_bounding_client_rect();
                    for index in 0..3 {
                        let x = rect.left() + rect.width() * (0.2 + index as f64 * 0.3);
                        let y = rect.top() + rect.height() * 0.5;
                        inject_at(&enter_state, x, y, 0.4);
                    }
                }));
                let move_state = state.clone();
                listeners.push(EventListener::new(button.as_ref(), "mousemove", move |event| {
                    let Some(event) = event.dyn_ref::<MouseEvent>() else {
                        return;
                    };
                    inject_at(&move_state, event.client_x() as f64, event.client_y() as f64, 0.15);
                }));
            }
        }
    }

    if state.borrow().opts.easter_eggs {
        let trigger_state = state.clone();
        let trigger = Closure::<dyn Fn()>::new(move || {
            trigger_state.borrow_mut().sim.trigger_about_blank();
        });
        let _ = js_sys::Reflect::set(
            &window(),
            &JsValue::from_str("__triggerAboutBlank"),
            trigger.as_ref(),
        );
        trigger.forget();

        let key_state = state.clone();
        listeners.push(EventListener::new(&document(), "keydown", move |event| {
            let Some(event) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
                return;
            };
            if event.ctrl_key() || event.alt_key() || event.meta_key() {
                return;
            }
            if document()
                .body()
                .map(|body| body.class_list().contains("inspect-open"))
                .unwrap_or(false)
            {
                return;
            }
            if let Some(target) = event.target().and_then(|target| target.dyn_into::<HtmlElement>().ok()) {
                if matches!(target.tag_name().as_str(), "INPUT" | "TEXTAREA")
                    || target.is_content_editable()
                {
                    return;
                }
            }
            let key = event.key();
            if key.chars().count() != 1 {
                return;
            }
            let trigger = key_state.borrow_mut().sim.key(&key).to_string();
            if trigger.is_empty() {
                return;
            }
            let delay = match trigger.as_str() {
                "42" => 3000,
                "cowsay" => 4000,
                _ => 5000,
            };
            let clear_state = key_state.clone();
            Timeout::new(delay, move || {
                clear_state.borrow_mut().sim.clear_egg();
            })
            .forget();
        }));

        let hash_state = state.clone();
        listeners.push(EventListener::new(&window(), "hashchange", move |_| {
            if window().location().hash().unwrap_or_default() != "#about:blank" {
                return;
            }
            hash_state.borrow_mut().sim.trigger_about_blank();
            replace_hash();
        }));
    }

    {
        let fonts_state = state.clone();
        spawn_local(async move {
            let fonts = document().fonts();
            if let Ok(ready) = fonts.ready() {
                let _ = wasm_bindgen_futures::JsFuture::from(ready).await;
            }
            let changed = {
                let mut water = fonts_state.borrow_mut();
                let old = water.char_width;
                water.measure_char_width();
                (water.char_width - old).abs() > 0.5
            };
            if changed {
                on_resize(&fonts_state);
            }
        });
    }
    {
        let later_state = state.clone();
        Timeout::new(1500, move || {
            let changed = {
                let mut water = later_state.borrow_mut();
                let old = water.char_width;
                water.measure_char_width();
                (water.char_width - old).abs() > 0.5
            };
            if changed {
                on_resize(&later_state);
            }
        })
        .forget();
    }

    state.borrow_mut().listeners.extend(listeners);
}

fn touch_at(list: &TouchList, index: u32) -> Option<(f64, f64)> {
    list.get(index)
        .map(|touch| (touch.client_x() as f64, touch.client_y() as f64))
}

fn inject_at(state: &Shared, screen_x: f64, screen_y: f64, intensity: f64) {
    let mut water = state.borrow_mut();
    if water.cols == 0 || water.rows == 0 {
        return;
    }
    let cw = water.char_px();
    let font = water.font_size();
    if cw <= 0.0 || font <= 0.0 {
        return;
    }
    let gi = (screen_x / cw)
        .round()
        .min(water.cols as f64 - 2.0)
        .max(1.0);
    let gj = (screen_y / font)
        .round()
        .min(water.rows as f64 - 2.0)
        .max(1.0);
    water.sim.inject_at(gi, gj, intensity);
}

fn schedule(state: &Shared) {
    let next = state.clone();
    let handle = request_animation_frame(move |timestamp| tick(&next, timestamp));
    state.borrow_mut().raf = Some(handle);
}

fn tick(state: &Shared, timestamp: f64) {
    if state.borrow().disposed {
        return;
    }
    let timestamp = if timestamp > 0.0 {
        timestamp
    } else {
        js_sys::Date::now()
    };
    let stepped = {
        let mut water = state.borrow_mut();
        let elapsed = (timestamp - water.last_timestamp).clamp(0.0, 250.0);
        water.last_timestamp = timestamp;
        water.pending_ms = (water.pending_ms + elapsed).min(FRAME_MS * MAX_CATCHUP);
        let mut stepped = false;
        while water.pending_ms >= FRAME_MS - 0.5 {
            water.sim.step();
            water.pending_ms = (water.pending_ms - FRAME_MS).max(0.0);
            stepped = true;
        }
        stepped
    };
    if stepped {
        render(state);
    }
    schedule(state);
}

fn resume(state: &Shared) {
    {
        let mut water = state.borrow_mut();
        if water.disposed || water.raf.is_some() {
            return;
        }
        water.last_timestamp = js_sys::Date::now();
        water.pending_ms = 0.0;
    }
    schedule(state);
}

fn pause(state: &Shared) {
    state.borrow_mut().raf = None;
}

fn build_api(state: Shared) -> JsValue {
    let api = js_sys::Object::new();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn(String, Option<bool>)>::new(move |mode: String, text: Option<bool>| {
            state.borrow_mut().sim.set_mode(&mode, text);
            render(&state);
        });
        set(&api, "setMode", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn(bool)>::new(move |visible: bool| {
            state.borrow_mut().sim.set_text_visible(visible);
            render(&state);
        });
        set(&api, "setTextVisible", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn() -> JsValue>::new(move || {
            let water = state.borrow();
            let grid = js_sys::Object::new();
            set(&grid, "cols", water.cols);
            set(&grid, "rows", water.rows);
            set(&grid, "charW", water.char_width);
            set(&grid, "rowH", water.font_size());
            set(&grid, "cellPx", CELL_PX);
            grid.into()
        });
        set(&api, "grid", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn() -> JsValue>::new(move || {
            JsValue::from_str(state.borrow().sim.ramp())
        });
        set(&api, "ramp", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn() -> JsValue>::new(move || field_of(&state));
        set(&api, "field", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn(JsValue)>::new(move |canvas: JsValue| {
            let Some(canvas) = canvas.dyn_into::<HtmlCanvasElement>().ok() else {
                return;
            };
            paint_current(&state, &canvas);
        });
        set(&api, "paintCurrent", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn(f64, f64) -> JsValue>::new(move |client_x: f64, client_y: f64| {
            sample_at(&state, client_x, client_y)
        });
        set(&api, "sampleAt", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn()>::new(move || {
            state.borrow_mut().sim.clear();
            render(&state);
        });
        set(&api, "clear", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn(f64, f64, Option<f64>, Option<f64>)>::new(
            move |ci: f64, cj: f64, amount: Option<f64>, radius: Option<f64>| {
                state
                    .borrow_mut()
                    .sim
                    .droplet(ci, cj, amount.unwrap_or(0.9), radius.unwrap_or(4.0));
                render(&state);
            },
        );
        set(&api, "droplet", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn(f64, f64, Option<f64>)>::new(
            move |ci: f64, cj: f64, strength: Option<f64>| {
                state
                    .borrow_mut()
                    .sim
                    .vortex(ci, cj, strength.unwrap_or(0.4));
                render(&state);
            },
        );
        set(&api, "vortex", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn()>::new(move || {
            state.borrow_mut().sim.stir();
            render(&state);
        });
        set(&api, "stir", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn()>::new(move || pause(&state));
        set(&api, "pause", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn()>::new(move || resume(&state));
        set(&api, "resume", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn()>::new(move || {
            on_resize(&state);
            render(&state);
        });
        set(&api, "resize", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn() -> JsValue>::new(move || {
            let running = state.borrow().raf.is_some();
            JsValue::from_bool(running)
        });
        set(&api, "isRunning", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn(JsValue)>::new(move |rects: JsValue| {
            apply_mask(&state, &rects);
            render(&state);
        });
        set(&api, "mask", closure.as_ref());
        closure
    };
    closure.forget();

    let closure = {
        let state = state.clone();
        let closure = Closure::<dyn Fn()>::new(move || {
            let mut water = state.borrow_mut();
            water.disposed = true;
            water.raf = None;
            water.typewriter = None;
            water.listeners.clear();
            if let Some(span) = water.char_span.take() {
                if let Some(parent) = span.parent_node() {
                    parent.remove_child(&span).ok();
                }
            }
        });
        set(&api, "dispose", closure.as_ref());
        closure
    };
    closure.forget();

    api.into()
}

fn field_of(state: &Shared) -> JsValue {
    let water = state.borrow();
    let length = water.cols.saturating_mul(water.rows);
    let memory = wasm_bindgen::memory().unchecked_into::<js_sys::WebAssembly::Memory>();
    let buffer = memory.buffer();
    let dens = js_sys::Float32Array::new_with_byte_offset_and_length(
        &buffer,
        water.sim.dens_ptr(),
        length,
    );
    let u = js_sys::Float32Array::new_with_byte_offset_and_length(
        &buffer,
        water.sim.u_ptr(),
        length,
    );
    let v = js_sys::Float32Array::new_with_byte_offset_and_length(
        &buffer,
        water.sim.v_ptr(),
        length,
    );
    let field = js_sys::Object::new();
    set(&field, "cols", water.cols);
    set(&field, "rows", water.rows);
    set(&field, "dens", dens);
    set(&field, "u", u);
    set(&field, "v", v);
    field.into()
}

fn sample_at(state: &Shared, client_x: f64, client_y: f64) -> JsValue {
    let water = state.borrow();
    if water.cols == 0 || water.rows == 0 {
        return JsValue::NULL;
    }
    let cw = water.char_px();
    let rh = water.font_size();
    if cw <= 0.0 || rh <= 0.0 {
        return JsValue::NULL;
    }
    let (x, y) = water.local_point(client_x, client_y);
    let i = (x / cw).floor();
    let j = (y / rh).floor();
    if i < 0.0 || j < 0.0 || i >= water.cols as f64 || j >= water.rows as f64 {
        return JsValue::NULL;
    }
    js_sys::JSON::parse(&water.sim.sample_json(i as i64, j as i64))
        .unwrap_or(JsValue::NULL)
}

fn apply_mask(state: &Shared, rects: &JsValue) {
    let mut water = state.borrow_mut();
    water.sim.clear_masks();
    let values = rects.dyn_ref::<js_sys::Array>();
    if let Some(values) = values {
        for value in values.iter() {
            if !value.is_object() {
                continue;
            }
            let row = option(&value, "row").and_then(|value| value.as_f64());
            let col = option(&value, "col").and_then(|value| value.as_f64());
            let rows = option(&value, "rows").and_then(|value| value.as_f64());
            let cols = option(&value, "cols").and_then(|value| value.as_f64());
            let (Some(row), Some(col), Some(rows), Some(cols)) = (row, col, rows, cols) else {
                continue;
            };
            if !row.is_finite() || !col.is_finite() || !rows.is_finite() || !cols.is_finite() {
                continue;
            }
            let fade = option(&value, "fade")
                .and_then(|value| value.as_f64())
                .unwrap_or(2.0);
            water.sim.push_mask(row, col, rows, cols, fade);
        }
    }
    water.sim.apply_masks();
}

fn paint_current(state: &Shared, canvas: &HtmlCanvasElement) {
    let water = state.borrow();
    let dpr = window().device_pixel_ratio().min(2.0).max(1.0);
    let width = if canvas.client_width() > 0 {
        canvas.client_width() as f64
    } else {
        window().inner_width().ok().and_then(|value| value.as_f64()).unwrap_or(0.0)
    };
    let height = if canvas.client_height() > 0 {
        canvas.client_height() as f64
    } else {
        window().inner_height().ok().and_then(|value| value.as_f64()).unwrap_or(0.0)
    };
    let pixel_width = (width * dpr).round() as u32;
    let pixel_height = (height * dpr).round() as u32;
    if canvas.width() != pixel_width {
        canvas.set_width(pixel_width);
    }
    if canvas.height() != pixel_height {
        canvas.set_height(pixel_height);
    }
    let Ok(Some(context)) = canvas.get_context("2d") else {
        return;
    };
    let Ok(context) = context.dyn_into::<web_sys::CanvasRenderingContext2d>() else {
        return;
    };
    context.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0).ok();
    context.clear_rect(0.0, 0.0, width, height);

    let cw = water.char_px();
    let rh = water.font_size();
    let segments = water.sim.current_segments();
    let bands: Vec<web_sys::Path2d> = (0..5).filter_map(|_| web_sys::Path2d::new().ok()).collect();
    if bands.len() < 5 {
        return;
    }
    let mut index = 0;
    while index + 4 < segments.len() {
        let band = (segments[index + 4] as i32).clamp(0, 4) as usize;
        let path = &bands[band];
        path.move_to(segments[index] as f64 * cw, segments[index + 1] as f64 * rh);
        path.line_to(segments[index + 2] as f64 * cw, segments[index + 3] as f64 * rh);
        index += 5;
    }
    context.set_line_width((cw.min(rh) * 0.1).max(0.75));
    context.set_line_cap("round");
    context.set_stroke_style_str("#fff");
    for (index, path) in bands.iter().enumerate() {
        context.set_global_alpha(0.08 + index as f64 * 0.065);
        context.stroke_with_path(path);
    }
    context.set_global_alpha(1.0);
}
