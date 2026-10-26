use std::cell::RefCell;
use std::rc::Rc;

use gloo::events::EventListener;
use gloo::render::{request_animation_frame, AnimationFrame};
use gloo::timers::callback::{Interval, Timeout};
use gloo::utils::{document, window};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{HtmlAudioElement, HtmlElement, HtmlImageElement, MouseEventInit};

pub fn start() {
    forward_pointer();
    redherring_audio();
    project_bunny();
    page_404();
    projects_parallax();
    embed_scene();
}

fn forward_pointer() {
    let window = window();
    let top = window.top().ok().flatten();
    if let Some(top) = top {
        if JsValue::from(top) == JsValue::from(window.clone()) {
            return;
        }
    }
    let Some(parent) = window.parent().ok().flatten() else {
        return;
    };
    let Some(host) = parent.document() else {
        return;
    };
    let Some(frame) = host.get_element_by_id("content-frame") else {
        return;
    };

    let listener = EventListener::new(&document(), "pointermove", move |event| {
        let Some(event) = event.dyn_ref::<web_sys::PointerEvent>() else {
            return;
        };
        if event.pointer_type() != "mouse" {
            return;
        }
        let rect = frame.get_bounding_client_rect();
        let init = MouseEventInit::new();
        init.set_client_x((rect.left() + event.client_x() as f64) as i32);
        init.set_client_y((rect.top() + event.client_y() as f64) as i32);
        init.set_bubbles(true);
        if let Ok(synthetic) = web_sys::MouseEvent::new_with_mouse_event_init_dict("mousemove", &init) {
            host.dispatch_event(&synthetic).ok();
        }
    });
    std::mem::forget(listener);
}

fn redherring_audio() {
    let Some(audio) = element("#rh-audio")
        .and_then(|element| element.dyn_into::<HtmlAudioElement>().ok())
    else {
        return;
    };
    let audio_for_events = audio.clone();
    let play = move || {
        if let Ok(promise) = audio_for_events.play() {
            let _ = JsFuture::from(promise);
        }
    };
    play();
    let target = document();
    let play_click = play.clone();
    let listener = EventListener::new(&target, "click", move |_| play_click());
    std::mem::forget(listener);
    let play_key = play.clone();
    let listener = EventListener::new(&target, "keydown", move |_| play_key());
    std::mem::forget(listener);
    let listener = EventListener::new(&target, "touchstart", move |_| play());
    std::mem::forget(listener);
}

fn project_bunny() {
    let Some(pre) = element("#specimen-pre") else { return };
    let state = Rc::new(RefCell::new((0.08_f64, -0.15_f64)));

    let draw = {
        let pre = pre.clone();
        let state = state.clone();
        move || {
            let (ax, ay) = *state.borrow();
            draw_bunny(&pre, ax, ay);
        }
    };

    let listener = {
        let state = state.clone();
        let draw = draw.clone();
        let pre_listener = pre.clone();
        EventListener::new(pre.as_ref(), "mousemove", move |event| {
            let Some(event) = event.dyn_ref::<web_sys::MouseEvent>() else {
                return;
            };
            let rect = pre_listener.get_bounding_client_rect();
            let ax = 0.08
                + ((event.client_y() as f64 - rect.top()) / rect.height() - 0.5) * 1.1;
            let ay = -0.15
                + ((event.client_x() as f64 - rect.left()) / rect.width() - 0.5) * 2.2;
            *state.borrow_mut() = (ax, ay);
            draw();
        })
    };
    std::mem::forget(listener);

    let draw_resize = draw.clone();
    let listener = EventListener::new(&window(), "resize", move |_| draw_resize());
    std::mem::forget(listener);

    let draw_ready = draw.clone();
    spawn_local(async move {
        if let Some(ready) = js_sys::Reflect::get(&window(), &JsValue::from_str("bunnyReady"))
            .ok()
            .and_then(|value| value.dyn_into::<js_sys::Promise>().ok())
        {
            let _ = JsFuture::from(ready).await;
        }
        draw_ready();
    });

    let draw_fonts = draw;
    spawn_local(async move {
        let fonts = document().fonts();
        if let Ok(ready) = fonts.ready() {
            let _ = JsFuture::from(ready).await;
        }
        draw_fonts();
    });
}

fn draw_bunny(pre: &HtmlElement, ax: f64, ay: f64) {
    let Some(ascii) = js_sys::Reflect::get(&window(), &JsValue::from_str("AsciiBunny"))
        .ok()
        .filter(|value| value.is_object())
    else {
        return;
    };
    let Some(render) = js_sys::Reflect::get(&ascii, &JsValue::from_str("render"))
        .ok()
        .and_then(|value| value.dyn_into::<js_sys::Function>().ok())
    else {
        return;
    };
    let font = window()
        .get_computed_style(pre)
        .ok()
        .flatten()
        .and_then(|style| style.get_property_value("font-size").ok())
        .and_then(|value| value.trim_end_matches("px").parse::<f64>().ok())
        .unwrap_or(10.0);
    let cols = ((pre.client_width() as f64) / (font * 0.6)).floor().clamp(24.0, 58.0);
    let rows = (cols * 0.6 / 1.18).round().max(12.0);

    let options = js_sys::Object::new();
    set(&options, "cols", cols);
    set(&options, "rows", rows);
    set(&options, "angleX", ax);
    set(&options, "angleY", ay);
    set(&options, "ramp", " .,-:~;=+*#%@");
    set(&options, "invert", false);

    if let Ok(lines) = render.call1(&ascii, &options) {
        if let Ok(lines) = lines.dyn_into::<js_sys::Array>() {
            pre.set_text_content(Some(&lines.join("\n").as_string().unwrap_or_default()));
        }
    }
}

struct Dvd {
    logo: HtmlElement,
    vx: f64,
    vy: f64,
    x: f64,
    y: f64,
    running: bool,
    raf: Option<AnimationFrame>,
}

fn page_404() {
    let Some(logo) = element("#dvd-logo") else { return };
    let Some(code) = element(".code") else { return };
    let state = Rc::new(RefCell::new(Dvd {
        logo: logo.clone(),
        vx: if js_sys::Math::random() > 0.5 { -1.1 } else { 1.1 },
        vy: if js_sys::Math::random() > 0.5 { -0.9 } else { 0.9 },
        x: 100.0,
        y: 200.0,
        running: false,
        raf: None,
    }));

    let state_click = state.clone();
    let listener = EventListener::new(code.as_ref(), "click", move |_| {
        let visible = state_click
            .borrow()
            .logo
            .class_list()
            .toggle("visible")
            .unwrap_or(false);
        if visible {
            start_dvd(state_click.clone());
        }
    });
    std::mem::forget(listener);

    let state_resize = state.clone();
    let listener = EventListener::new(&window(), "resize", move |_| {
        let mut state = state_resize.borrow_mut();
        let view_w = window().inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
        let view_h = window().inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
        let rect = state.logo.get_bounding_client_rect();
        if state.x + rect.width() > view_w {
            state.x = view_w - rect.width();
        }
        if state.y + rect.height() > view_h {
            state.y = view_h - rect.height();
        }
        state.x = state.x.max(0.0);
        state.y = state.y.max(0.0);
    });
    std::mem::forget(listener);

    page_404_scene();
}

fn start_dvd(state: Rc<RefCell<Dvd>>) {
    if state.borrow().running {
        return;
    }
    state.borrow_mut().running = true;
    let handle = schedule_dvd(state.clone());
    state.borrow_mut().raf = Some(handle);
}

fn schedule_dvd(state: Rc<RefCell<Dvd>>) -> AnimationFrame {
    request_animation_frame(move |_| {
        {
            let mut dvd = state.borrow_mut();
            tick_dvd(&mut dvd);
        }
        let next = schedule_dvd(state.clone());
        state.borrow_mut().raf = Some(next);
    })
}

fn tick_dvd(dvd: &mut Dvd) {
    let view_w = window().inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
    let view_h = window().inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
    let rect = dvd.logo.get_bounding_client_rect();
    let width = rect.width();
    let height = rect.height();
    dvd.x += dvd.vx;
    dvd.y += dvd.vy;
    if dvd.x <= 0.0 {
        dvd.x = 0.0;
        dvd.vx = -dvd.vx;
    } else if dvd.x + width >= view_w {
        dvd.x = view_w - width;
        dvd.vx = -dvd.vx;
    }
    if dvd.y <= 0.0 {
        dvd.y = 0.0;
        dvd.vy = -dvd.vy;
    } else if dvd.y + height >= view_h {
        dvd.y = view_h - height;
        dvd.vy = -dvd.vy;
    }
    dvd.logo
        .style()
        .set_property("transform", &format!("translate({:.1}px,{:.1}px)", dvd.x, dvd.y))
        .ok();
}

struct Scene {
    earth: HtmlElement,
    ship: HtmlElement,
    glass: HtmlElement,
    depths: [f64; 3],
    tx: f64,
    ty: f64,
    cx: f64,
    cy: f64,
    last_move: f64,
    reduce: bool,
}

fn page_404_scene() {
    let Some(scene) = element("#scene") else { return };
    let Some(earth) = element("#layer-earth") else { return };
    let Some(ship) = element("#layer-ship") else { return };
    let Some(glass) = element("#layer-glass") else { return };
    if document().ready_state() == "complete" {
        scene.class_list().add_1("ready").ok();
    } else {
        let scene = scene.clone();
        let listener = EventListener::new(&window(), "load", move |_| {
            scene.class_list().add_1("ready").ok();
        });
        std::mem::forget(listener);
    }

    let reduce = window()
        .match_media("(prefers-reduced-motion: reduce)")
        .ok()
        .flatten()
        .map(|query| query.matches())
        .unwrap_or(false);
    let state = Rc::new(RefCell::new(Scene {
        earth,
        ship,
        glass,
        depths: [0.0; 3],
        tx: 0.0,
        ty: 0.0,
        cx: 0.0,
        cy: 0.0,
        last_move: -1e9,
        reduce,
    }));
    resize_scene(&mut state.borrow_mut());
    let state_resize = state.clone();
    let listener = EventListener::new(&window(), "resize", move |_| {
        resize_scene(&mut state_resize.borrow_mut());
    });
    std::mem::forget(listener);

    let state_move = state.clone();
    let listener = EventListener::new(&window(), "mousemove", move |event| {
        let Some(event) = event.dyn_ref::<web_sys::MouseEvent>() else {
            return;
        };
        let mut state = state_move.borrow_mut();
        state.tx = (event.client_x() as f64 / window().inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(1.0)) * 2.0 - 1.0;
        state.ty = (event.client_y() as f64 / window().inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(1.0)) * 2.0 - 1.0;
        state.last_move = js_sys::Date::now();
    });
    std::mem::forget(listener);

    let handle = schedule_scene(state.clone());
    std::mem::forget(handle);
}

fn schedule_scene(state: Rc<RefCell<Scene>>) -> AnimationFrame {
    request_animation_frame(move |time| {
        {
            let mut scene = state.borrow_mut();
            tick_scene(&mut scene, time);
        }
        schedule_scene(state.clone());
    })
}

fn tick_scene(scene: &mut Scene, time: f64) {
    if !scene.reduce && time - scene.last_move > 2200.0 {
        scene.tx = (time * 0.0005).sin() * 0.75;
        scene.ty = (time * 0.0004).cos() * 0.75;
    }
    scene.cx += (scene.tx - scene.cx) * 0.09;
    scene.cy += (scene.ty - scene.cy) * 0.09;
    for (element, depth) in [
        (&scene.earth, scene.depths[0]),
        (&scene.ship, scene.depths[1]),
        (&scene.glass, scene.depths[2]),
    ] {
        element
            .style()
            .set_property(
                "transform",
                &format!(
                    "translate3d({:.2}px,{:.2}px,0)",
                    scene.cx * depth,
                    scene.cy * depth
                ),
            )
            .ok();
    }
}

fn resize_scene(scene: &mut Scene) {
    let size = window()
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
        .min(
            window()
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
        );
    scene.depths = [size * 0.002, size * 0.010, size * 0.020];
}

fn projects_parallax() {
    let Some(branch) = element(".branch") else { return };
    let Some(tree) = element(".tree") else { return };
    let by = Rc::new(RefCell::new(0.0_f64));
    let ty = Rc::new(RefCell::new(0.0_f64));

    let branch_move = branch.clone();
    let tree_move = tree.clone();
    let by_move = by.clone();
    let ty_move = ty.clone();
    let listener = EventListener::new(&window(), "mousemove", move |event| {
        let Some(event) = event.dyn_ref::<web_sys::MouseEvent>() else {
            return;
        };
        let ny = (event.client_y() as f64
            / window().inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(1.0)
            - 0.5)
            * 2.0;
        let mut by = by_move.borrow_mut();
        let mut ty = ty_move.borrow_mut();
        *by += (-ny * 20.0 - *by) * 0.3;
        *ty += (ny * 20.0 - *ty) * 0.3;
        branch_move
            .style()
            .set_property("transform", &format!("translate3d(0,{:.2}px,0)", *by))
            .ok();
        tree_move
            .style()
            .set_property("transform", &format!("translate3d(0,{:.2}px,0)", *ty))
            .ok();
    });
    std::mem::forget(listener);

    let branch_reset = branch.clone();
    let tree_reset = tree.clone();
    let by_reset = by.clone();
    let ty_reset = ty.clone();
    let listener = EventListener::new(&window(), "mouseleave", move |_| {
        *by_reset.borrow_mut() = 0.0;
        *ty_reset.borrow_mut() = 0.0;
        branch_reset
            .style()
            .set_property("transform", "translate3d(0px,0px,0px)")
            .ok();
        tree_reset
            .style()
            .set_property("transform", "translate3d(0px,0px,0px)")
            .ok();
    });
    std::mem::forget(listener);
}

fn embed_scene() {
    let Some(mount) = element("#embed-water") else { return };
    let sim: Rc<RefCell<Option<JsValue>>> = Rc::new(RefCell::new(None));
    let started = Rc::new(RefCell::new(false));

    let arm: Rc<dyn Fn()> = {
        let sim = sim.clone();
        let started = started.clone();
        Rc::new(move || {
            if *started.borrow() {
                return;
            }
            *started.borrow_mut() = true;
            let sim = sim.clone();
            let mount = mount.clone();
            spawn_local(async move {
                if let Some(ready) = js_sys::Reflect::get(&window(), &JsValue::from_str("waterSimReady"))
                    .ok()
                    .and_then(|value| value.dyn_into::<js_sys::Promise>().ok())
                {
                    let _ = JsFuture::from(ready).await;
                }
                let Some(create) = js_sys::Reflect::get(&window(), &JsValue::from_str("createWaterSim"))
                    .ok()
                    .and_then(|value| value.dyn_into::<js_sys::Function>().ok())
                else {
                    return;
                };
                let options = js_sys::Object::new();
                let _ = js_sys::Reflect::set(&options, &JsValue::from_str("mount"), &mount);
                let _ = js_sys::Reflect::set(
                    &options,
                    &JsValue::from_str("buttons"),
                    &js_sys::Array::new(),
                );
                let _ = js_sys::Reflect::set(&options, &JsValue::from_str("text"), &JsValue::FALSE);
                let size = Closure::<dyn Fn() -> JsValue>::new(|| {
                    let size = js_sys::Object::new();
                    let _ = js_sys::Reflect::set(&size, &JsValue::from_str("width"), &JsValue::from_f64(1200.0));
                    let _ = js_sys::Reflect::set(&size, &JsValue::from_str("height"), &JsValue::from_f64(600.0));
                    size.into()
                });
                let _ = js_sys::Reflect::set(&options, &JsValue::from_str("size"), size.as_ref());
                size.forget();
                let Ok(created) = create.call1(&JsValue::NULL, &options) else {
                    return;
                };
                *sim.borrow_mut() = Some(created.clone());
                seed_embed(&created);
                let sim = sim.clone();
                let mut tick = 0_u32;
                let mut time = 0.0_f64;
                Interval::new(50, move || {
                    if let Some(sim) = sim.borrow().as_ref() {
                        tick_embed(sim, &mut tick, &mut time);
                    }
                })
                .forget();
            });
        })
    };

    let arm_for_function = arm.clone();
    let function = Closure::<dyn Fn() -> JsValue>::new(move || {
        arm_for_function();
        js_sys::Promise::resolve(&JsValue::UNDEFINED).into()
    });
    let _ = js_sys::Reflect::set(&window(), &JsValue::from_str("__arm"), function.as_ref());
    function.forget();

    Timeout::new(3000, move || arm()).forget();
}

fn seed_embed(sim: &JsValue) {
    let spots = [
        (0.22, 0.34), (0.38, 0.24), (0.56, 0.30), (0.74, 0.40),
        (0.30, 0.56), (0.50, 0.64), (0.68, 0.58), (0.84, 0.62),
        (0.46, 0.44),
    ];
    let (cols, rows) = grid(sim);
    for (index, (x, y)) in spots.iter().enumerate() {
        let col = cols * x;
        let row = rows * y;
        call(sim, "droplet", &[col.into(), row.into(), 0.42.into(), 1.0.into()]);
        call(
            sim,
            "vortex",
            &[col.into(), row.into(), (if index % 2 == 0 { 0.22 } else { -0.22 }).into()],
        );
    }
}

fn tick_embed(sim: &JsValue, tick: &mut u32, time: &mut f64) {
    *time += 0.045;
    *tick += 1;
    let (cols, rows) = grid(sim);
    for index in 0..3 {
        let phase = *time + index as f64 * 2.1;
        let col = cols * (0.50 + (phase * 1.1).sin() * 0.31);
        let row = rows * (0.50 + (phase * 2.2 + index as f64 * 1.7).sin() * 0.26);
        call(sim, "droplet", &[col.into(), row.into(), 0.16.into(), 1.0.into()]);
        if *tick % 5 == 3 {
            let strength = if index % 2 == 0 { 0.34 } else { -0.34 };
            call(sim, "vortex", &[col.into(), row.into(), strength.into()]);
        }
    }
}

fn grid(sim: &JsValue) -> (f64, f64) {
    let grid = call(sim, "grid", &[]);
    (
        js_sys::Reflect::get(&grid, &JsValue::from_str("cols"))
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        js_sys::Reflect::get(&grid, &JsValue::from_str("rows"))
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
    )
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

fn set(target: &js_sys::Object, key: &str, value: impl Into<JsValue>) {
    let _ = js_sys::Reflect::set(target, &JsValue::from_str(key), &value.into());
}

fn element(selector: &str) -> Option<HtmlElement> {
    document()
        .query_selector(selector)
        .ok()
        .flatten()
        .and_then(|node| node.dyn_into::<HtmlElement>().ok())
}

#[allow(dead_code)]
fn image(src: &str) -> Option<HtmlImageElement> {
    let image = document().create_element("img").ok()?.dyn_into::<HtmlImageElement>().ok()?;
    image.set_src(src);
    Some(image)
}
