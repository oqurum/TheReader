use std::rc::Rc;

use chrono::{Duration, Utc};
use wasm_bindgen::UnwrapThrowExt;
use web_sys::{DomRect, HtmlElement, MouseEvent};
use yew::{
    function_component, hook, html, use_effect_with_deps, use_mut_ref, use_node_ref, use_state_eq,
    Callback, Html, NodeRef, Properties, UseStateHandle,
};
use yew_hooks::{use_event, use_swipe, UseSwipeDirection};

#[derive(Debug)]
pub enum OverlayEvent {
    // Mouse Release
    Release {
        instant: Option<Duration>,

        x: i32,
        y: i32,

        width: i32,
        height: i32,
    },

    // Mouse hovering over overlay.
    Hover {
        x: i32,
        y: i32,
    },

    // Mouse Drag
    Drag {
        type_of: DragType,
        instant: Option<Duration>,
        coords_start: (i32, i32),
        coords_end: (i32, i32),
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragType {
    Up(usize),
    Right(usize),
    Down(usize),
    Left(usize),
    None,
}

#[derive(PartialEq, Properties)]
pub struct ViewOverlayProps {
    pub event: Callback<OverlayEvent>,
}

#[function_component(ViewOverlay)]
pub fn _view_overlay(props: &ViewOverlayProps) -> Html {
    let node = use_node_ref();
    let state = use_swipe(node.clone());
    let state2 = use_mouse(node.clone());

    let time_down = use_state_eq(Utc::now);

    let curr_event_state = use_state_eq(|| false);

    {
        // Swipe
        let event = props.event.clone();
        let curr_event_state = curr_event_state.clone();
        let time_down = time_down.clone();

        let canvas_node = node.clone();

        use_effect_with_deps(
            move |(swiping, direction, length_x, length_y, coords_start, coords_end)| {
                let distance = match **direction {
                    UseSwipeDirection::Left => length_x.abs(),
                    UseSwipeDirection::Right => length_x.abs(),
                    UseSwipeDirection::Up => length_y.abs(),
                    UseSwipeDirection::Down => length_y.abs(),
                    UseSwipeDirection::None => 0,
                } as usize;

                let direction = match **direction {
                    UseSwipeDirection::Left => DragType::Left(distance),
                    UseSwipeDirection::Right => DragType::Right(distance),
                    UseSwipeDirection::Up => DragType::Up(distance),
                    UseSwipeDirection::Down => DragType::Down(distance),
                    UseSwipeDirection::None => DragType::None,
                };

                if **swiping {
                    if !*curr_event_state {
                        time_down.set(Utc::now());
                    }

                    curr_event_state.set(true);

                    event.emit(OverlayEvent::Drag {
                        type_of: direction,
                        instant: None,
                        coords_start: **coords_start,
                        coords_end: **coords_end,
                    });
                } else if *curr_event_state {
                    curr_event_state.set(false);

                    let (x, y) = **coords_start;

                    let (width, height) = {
                        let node = canvas_node.cast::<HtmlElement>().unwrap();

                        (node.client_width(), node.client_height())
                    };

                    event.emit(OverlayEvent::Release {
                        instant: Some(Utc::now().signed_duration_since(*time_down)),
                        x,
                        y,

                        width,
                        height,
                    });
                }
                || ()
            },
            (
                state.swiping,
                state.direction,
                state.length_x,
                state.length_y,
                state.coords_start,
                state.coords_end,
            ),
        );
    }

    {
        // Mouse
        let event = props.event.clone();

        let canvas_node = node.clone();

        use_effect_with_deps(
            move |handle| {
                if *handle.dragging || *curr_event_state {
                    let distance = match *handle.direction {
                        UseSwipeDirection::Left => handle.length_x.abs(),
                        UseSwipeDirection::Right => handle.length_x.abs(),
                        UseSwipeDirection::Up => handle.length_y.abs(),
                        UseSwipeDirection::Down => handle.length_y.abs(),
                        UseSwipeDirection::None => 0,
                    } as usize;

                    let direction = match *handle.direction {
                        UseSwipeDirection::Left => DragType::Left(distance),
                        UseSwipeDirection::Right => DragType::Right(distance),
                        UseSwipeDirection::Up => DragType::Up(distance),
                        UseSwipeDirection::Down => DragType::Down(distance),
                        UseSwipeDirection::None => DragType::None,
                    };

                    // If we're dragging the mouse down and it's registered as moving.
                    if *handle.dragging {
                        if !*curr_event_state {
                            time_down.set(Utc::now());
                        }

                        curr_event_state.set(true);

                        event.emit(OverlayEvent::Drag {
                            type_of: direction,
                            instant: None,
                            coords_start: *handle.coords_start,
                            coords_end: handle.coords_end.unwrap_or_default(),
                        });
                    } else if *curr_event_state {
                        curr_event_state.set(false);

                        let (width, height) = {
                            let node = canvas_node.cast::<HtmlElement>().unwrap();

                            (node.client_width(), node.client_height())
                        };

                        event.emit(OverlayEvent::Release {
                            instant: Some(Utc::now().signed_duration_since(*time_down)),
                            x: handle.coords_start.0,
                            y: handle.coords_start.1,

                            width, height,
                        });
                    }
                } else {
                    event.emit(OverlayEvent::Hover {
                        x: handle.coords_start.0,
                        y: handle.coords_start.1,
                    });
                }

                || ()
            },
            state2,
        );
    }

    // TODO: Implement Mouse Move

    html! {
        <canvas class="view-overlay" ref={ node } style="user-select: none;" />
    }
}

// Based off Swipe

#[derive(Debug, PartialEq)]
pub struct UseMouseHandle {
    pub dragging: UseStateHandle<bool>,
    pub moving: UseStateHandle<bool>,

    pub direction: UseStateHandle<UseSwipeDirection>,

    pub coords_start: UseStateHandle<(i32, i32)>,
    pub coords_end: UseStateHandle<Option<(i32, i32)>>,

    pub length_x: UseStateHandle<i32>,
    pub length_y: UseStateHandle<i32>,
}

impl Clone for UseMouseHandle {
    fn clone(&self) -> Self {
        Self {
            dragging: self.dragging.clone(),
            moving: self.moving.clone(),
            direction: self.direction.clone(),
            coords_start: self.coords_start.clone(),
            coords_end: self.coords_end.clone(),
            length_x: self.length_x.clone(),
            length_y: self.length_y.clone(),
        }
    }
}

#[hook]
pub fn use_mouse(node: NodeRef) -> UseMouseHandle {
    let node_bb = use_mut_ref(|| DomRect::new().unwrap_throw());
    let dragging = use_state_eq(|| false);
    let moving = use_state_eq(|| false);
    let direction = use_state_eq(|| UseSwipeDirection::None);
    let coords_start = use_state_eq(|| (0, 0));
    let coords_end = use_state_eq(|| Option::<(i32, i32)>::None);
    let length_x = use_state_eq(|| 0);
    let length_y = use_state_eq(|| 0);

    let threshold = 5;

    let diff_x = {
        let coords_start = coords_start.clone();
        let coords_end = coords_end.clone();

        Rc::new(move || {
            if let Some(coords_end) = *coords_end {
                coords_start.0 - coords_end.0
            } else {
                0
            }
        })
    };

    let diff_y = {
        let coords_start = coords_start.clone();
        let coords_end = coords_end.clone();

        Rc::new(move || {
            if let Some(coords_end) = *coords_end {
                coords_start.1 - coords_end.1
            } else {
                0
            }
        })
    };

    let threshold_exceeded = {
        let diff_x = diff_x.clone();
        let diff_y = diff_y.clone();

        Rc::new(move || diff_x().abs().max(diff_y().abs()) >= threshold)
    };

    {
        let node_bb = node_bb.clone();
        let node = node.clone();

        use_event(node.clone(), "mouseenter", move |_: MouseEvent| {
            *node_bb.borrow_mut() = node
                .cast::<HtmlElement>()
                .unwrap_throw()
                .get_bounding_client_rect();
        });
    }

    {
        let coords_end = coords_end.clone();
        let dragging = dragging.clone();

        use_event(node.clone(), "mousedown", move |_: MouseEvent| {
            coords_end.set(None);
            dragging.set(true);
        });
    }

    {
        let coords_start = coords_start.clone();
        let coords_end = coords_end.clone();
        let moving = moving.clone();
        let length_x = length_x.clone();
        let length_y = length_y.clone();
        let direction = direction.clone();
        let dragging = dragging.clone();

        use_event(node.clone(), "mousemove", move |e: MouseEvent| {
            let node_bb = node_bb.borrow();

            let x = e.x() - node_bb.left() as i32;
            let y = e.y() - node_bb.top() as i32;

            if !*dragging {
                coords_start.set((x, y));
                coords_end.set(None);
            } else {
                coords_end.set(Some((x, y)));

                length_x.set(diff_x());
                length_y.set(diff_y());

                if !*moving && threshold_exceeded() {
                    moving.set(true);
                }

                if !threshold_exceeded() {
                    direction.set(UseSwipeDirection::None);
                } else if diff_x().abs() > diff_y().abs() {
                    if diff_x() > 0 {
                        direction.set(UseSwipeDirection::Left);
                    } else {
                        direction.set(UseSwipeDirection::Right);
                    }
                } else if diff_y() > 0 {
                    direction.set(UseSwipeDirection::Up);
                } else {
                    direction.set(UseSwipeDirection::Down);
                }
            }
        });
    }

    {
        let moving = moving.clone();
        let direction = direction.clone();
        let dragging = dragging.clone();

        use_event(node.clone(), "mouseup", move |_: MouseEvent| {
            moving.set(false);
            dragging.set(false);
            direction.set(UseSwipeDirection::None);
        });
    }

    {
        // Copy of mouseup
        let moving = moving.clone();
        let direction = direction.clone();
        let dragging = dragging.clone();

        use_event(node, "mouseleave", move |_: MouseEvent| {
            moving.set(false);
            dragging.set(false);
            direction.set(UseSwipeDirection::None);
        });
    }

    UseMouseHandle {
        dragging,
        moving,
        direction,
        coords_start,
        coords_end,
        length_x,
        length_y,
    }
}
