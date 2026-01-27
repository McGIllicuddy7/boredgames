use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{Arc, Mutex},
};

use raylib::{color::Color, prelude::RaylibDrawHandle};

use crate::{
    rtils::rtils_useful::{Exception, Immutable, SharedList, Throws},
    tgui::{ComputedBoundary, GuiObject, TGui, TGuiOutput, get_string_bounds},
};

use crate::throw;
#[repr(transparent)]
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Copy, Hash)]

pub struct ElementId {
    v: u32,
}
impl Default for ElementId {
    fn default() -> Self {
        Self::new()
    }
}

impl ElementId {
    pub const fn new() -> Self {
        Self { v: 0 }
    }
    pub const fn is_valid(&self) -> bool {
        self.v != 0
    }
    pub const fn inner(&self) -> u32 {
        self.v
    }
}

pub trait StateView: 'static {
    fn mutated(&self, id: ElementId, gui: &TransGui) -> bool;
    fn update_view(&self, id: ElementId, gui: &mut TransGui);
}

pub struct TransGui {
    elements: BTreeMap<ElementId, TransGuiElement>,
    roots: Vec<ElementId>,
    fg_color: Color,
    bg_color: Color,
    name_table: HashMap<String, ElementId>,
    gui: TGui,
    scrollbar_outputs: HashMap<ElementId, TGuiOutput<i32>>,
    button_outputs: HashMap<ElementId, TGuiOutput<bool>>,
    box_outputs: HashMap<ElementId, TGuiOutput<ComputedBoundary>>,
    mutated: bool,
    modifications: usize,
    hidden: HashSet<ElementId>,
    state_views: HashMap<ElementId, Arc<dyn StateView>>,
}

pub type GuiUpdateFn = dyn FnMut(&mut TransGui, ElementId) + 'static;
#[derive(Clone)]
pub enum TransGuiElement {
    String {
        s: String,
        color: Color,
        parent: Immutable<ElementId>,
    },
    Box {
        h: i32,
        w: i32,
        color: Color,
        parent: Immutable<ElementId>,
    },
    Button {
        color: Color,
        on_pressed: Arc<Mutex<GuiUpdateFn>>,
        parent: Immutable<ElementId>,
        text: String,
    },
    Container {
        children: Immutable<Vec<ElementId>>,
        horizontal: bool,
        parent: Immutable<ElementId>,
        color: Color,
        upside_down: bool,
    },
    ScrollBox {
        scroll_amount: i32,
        w: i32,
        h: i32,
        children: Immutable<Vec<ElementId>>,
        parent: Immutable<ElementId>,
        color: Color,
        upside_down: bool,
    },
    BoxedGuiObject {
        obj: Box<dyn GuiObject>,
        parent: Immutable<ElementId>,
    },
}

impl TransGuiElement {
    pub fn get_parent(&self) -> ElementId {
        match self {
            TransGuiElement::String {
                s: _,
                color: _,
                parent,
            } => *parent.get(),
            TransGuiElement::Box {
                h: _,
                w: _,
                color: _,
                parent,
            } => *parent.get(),
            TransGuiElement::Button {
                color: _,
                on_pressed: _,
                parent,
                text: _,
            } => *parent.get(),
            TransGuiElement::Container {
                children: _,
                horizontal: _,
                parent,
                color: _,
                upside_down: _,
            } => *parent.get(),
            TransGuiElement::ScrollBox {
                scroll_amount: _,
                w: _,
                h: _,
                children: _,
                parent,
                color: _,
                upside_down: _,
            } => *parent.get(),
            TransGuiElement::BoxedGuiObject { obj: _, parent } => *parent.get(),
        }
    }
    fn set_parent(&mut self, new_parent: ElementId) {
        unsafe {
            match self {
                TransGuiElement::String {
                    s: _,
                    color: _,
                    parent,
                } => {
                    *parent.get_mut() = new_parent;
                }

                TransGuiElement::Box {
                    h: _,
                    w: _,
                    color: _,
                    parent,
                } => {
                    *parent.get_mut() = new_parent;
                }
                TransGuiElement::Button {
                    color: _,
                    on_pressed: _,
                    parent,
                    text: _,
                } => {
                    *parent.get_mut() = new_parent;
                }
                TransGuiElement::Container {
                    children: _,
                    horizontal: _,
                    parent,
                    color: _,
                    upside_down: _,
                } => {
                    *parent.get_mut() = new_parent;
                }
                TransGuiElement::ScrollBox {
                    scroll_amount: _,
                    w: _,
                    h: _,
                    children: _,
                    parent,
                    color: _,
                    upside_down: _,
                } => {
                    *parent.get_mut() = new_parent;
                }
                TransGuiElement::BoxedGuiObject { obj: _, parent } => {
                    *parent.get_mut() = new_parent;
                }
            }
        }
    }
}

impl Default for TransGui {
    fn default() -> Self {
        Self::new()
    }
}

impl TransGui {
    pub fn new() -> Self {
        Self {
            elements: BTreeMap::new(),
            roots: Vec::new(),
            fg_color: Color::GREEN,
            bg_color: Color::GREEN,
            name_table: HashMap::new(),
            gui: TGui::new(),
            mutated: false,
            scrollbar_outputs: HashMap::new(),
            button_outputs: HashMap::new(),
            modifications: 0,
            hidden: HashSet::new(),
            state_views: HashMap::new(),
            box_outputs: HashMap::new(),
        }
    }

    pub fn new_element(&mut self, e: TransGuiElement) -> ElementId {
        self.mutated = true;
        self.modifications += 1;
        let mut idx = 0;
        for i in 1..u32::MAX {
            let id = ElementId { v: i };
            if !self.elements.contains_key(&id) {
                idx = i;
                break;
            }
        }
        assert!(idx != 0);
        let out = ElementId { v: idx };
        self.elements.insert(out, e);
        out
    }

    pub fn new_text(&mut self, text: impl Into<String>) -> ElementId {
        self.mutated = true;
        self.modifications += 1;
        let e = TransGuiElement::String {
            s: text.into(),
            color: self.fg_color,
            parent: Immutable::new(ElementId::new()),
        };
        self.new_element(e)
    }

    pub fn new_box(&mut self, h: i32, w: i32) -> ElementId {
        self.mutated = true;
        self.modifications += 1;
        let e = TransGuiElement::Box {
            h,
            w,
            color: self.fg_color,
            parent: Immutable::new(ElementId::new()),
        };
        self.new_element(e)
    }

    pub fn new_button(
        &mut self,
        on_click: impl FnMut(&mut TransGui, ElementId) + 'static,
        text: impl Into<String>,
    ) -> ElementId {
        self.mutated = true;
        self.modifications += 1;
        let e = TransGuiElement::Button {
            color: self.fg_color,
            on_pressed: Arc::new(Mutex::new(on_click)),
            parent: Immutable::new(ElementId::new()),
            text: text.into(),
        };
        self.new_element(e)
    }

    pub fn new_section(&mut self) -> ElementId {
        self.mutated = true;
        self.modifications += 1;
        let e = TransGuiElement::Container {
            children: Immutable::new(Vec::new()),
            horizontal: false,
            parent: Immutable::new(ElementId::new()),
            color: self.bg_color,
            upside_down: false,
        };
        self.new_element(e)
    }

    pub fn new_section_upside_down(&mut self) -> ElementId {
        self.mutated = true;
        self.modifications += 1;
        let e = TransGuiElement::Container {
            children: Immutable::new(Vec::new()),
            horizontal: false,
            parent: Immutable::new(ElementId::new()),
            color: self.bg_color,
            upside_down: true,
        };
        self.new_element(e)
    }
    pub fn new_horizontal_section(&mut self) -> ElementId {
        self.mutated = true;
        self.modifications += 1;
        let e = TransGuiElement::Container {
            children: Immutable::new(Vec::new()),
            horizontal: true,
            parent: Immutable::new(ElementId::new()),
            color: self.bg_color,
            upside_down: false,
        };
        self.new_element(e)
    }

    pub fn new_scroll_box(&mut self, w: i32, h: i32) -> ElementId {
        self.mutated = true;
        self.modifications += 1;
        let e = TransGuiElement::ScrollBox {
            scroll_amount: 0,
            w,
            h,
            children: Immutable::new(Vec::new()),
            parent: Immutable::new(ElementId::new()),
            color: self.bg_color,
            upside_down: false,
        };
        self.new_element(e)
    }

    pub fn new_scroll_box_upside_down(&mut self, w: i32, h: i32) -> ElementId {
        self.mutated = true;
        self.modifications += 1;
        let e = TransGuiElement::ScrollBox {
            scroll_amount: 0,
            w,
            h,
            children: Immutable::new(Vec::new()),
            parent: Immutable::new(ElementId::new()),
            color: self.bg_color,
            upside_down: true,
        };
        self.new_element(e)
    }

    pub fn new_gui_object(&mut self, obj: Box<dyn GuiObject>) -> ElementId {
        self.mutated = true;
        self.modifications += 1;
        let e = TransGuiElement::BoxedGuiObject {
            obj,
            parent: Immutable::new(ElementId::new()),
        };
        self.new_element(e)
    }

    pub fn attach_to_doc(&mut self, id: ElementId) {
        self.modifications += 1;
        self.mutated = true;
        self.detach_element(id);
        self.roots.push(id);
    }

    pub fn attach_to_element(&mut self, id: ElementId, parent_to: ElementId) {
        self.mutated = true;
        self.modifications += 1;
        self.detach_element(id);
        let prs = self.get_element(parent_to).unwrap();
        unsafe {
            match prs {
                TransGuiElement::Container {
                    children,
                    horizontal: _,
                    parent: _,
                    color: _,
                    upside_down: _,
                } => {
                    children.get_mut().push(id);
                }
                TransGuiElement::ScrollBox {
                    scroll_amount: _,
                    w: _,
                    h: _,
                    children,
                    parent: _,
                    color: _,
                    upside_down: _,
                } => {
                    children.get_mut().push(id);
                }
                _ => {
                    todo!()
                }
            }
            match self.get_element(id).unwrap() {
                TransGuiElement::String {
                    s: _,
                    color: _,
                    parent,
                } => {
                    *parent.get_mut() = parent_to;
                }
                TransGuiElement::Box {
                    h: _,
                    w: _,
                    color: _,
                    parent,
                } => {
                    *parent.get_mut() = parent_to;
                }
                TransGuiElement::Button {
                    color: _,
                    on_pressed: _,
                    parent,
                    text: _,
                } => {
                    *parent.get_mut() = parent_to;
                }
                TransGuiElement::Container {
                    children: _,
                    horizontal: _,
                    parent,
                    color: _,
                    upside_down: _,
                } => {
                    *parent.get_mut() = parent_to;
                }
                TransGuiElement::ScrollBox {
                    scroll_amount: _,
                    w: _,
                    h: _,
                    children: _,
                    parent,
                    color: _,
                    upside_down: _,
                } => {
                    *parent.get_mut() = parent_to;
                }
                TransGuiElement::BoxedGuiObject { obj: _, parent } => {
                    *parent.get_mut() = parent_to;
                }
            }
        }
    }

    pub fn detach_element(&mut self, id: ElementId) {
        if !id.is_valid() {
            return;
        }
        self.mutated = true;
        self.modifications += 1;
        let element = self.get_element(id).unwrap();
        let parent = element.get_parent();
        let is_valid = parent.is_valid();
        element.set_parent(ElementId::new());
        if is_valid {
            let pr = self.get_element(parent).unwrap();
            unsafe {
                match pr {
                    TransGuiElement::Container {
                        children,
                        horizontal: _,
                        parent: _,
                        color: _,
                        upside_down: _,
                    } => {
                        let mut idx = -1;
                        for i in 0..children.get().len() {
                            if children.get_mut()[i] == id {
                                idx = i as i32;
                                break;
                            }
                        }
                        if idx == -1 {
                            todo!()
                        } else {
                            let prev_len = children.get().len();
                            children.get_mut().remove(idx as usize);
                            assert!(children.get().len() == prev_len - 1);
                        }
                    }
                    TransGuiElement::ScrollBox {
                        scroll_amount: _,
                        w: _,
                        h: _,
                        children,
                        parent: _,
                        color: _,
                        upside_down: _,
                    } => {
                        let mut idx = -1;
                        for i in 0..children.get().len() {
                            if children.get_mut()[i] == id {
                                idx = i as i32;
                                break;
                            }
                        }
                        if idx == -1 {
                            todo!()
                        } else {
                            let prev_len = children.get().len();
                            children.get_mut().remove(idx as usize);
                            assert!(children.get().len() == prev_len - 1);
                        }
                    }
                    _ => {
                        todo!()
                    }
                }
            }
        }
        let mut idx = -1;
        for i in 0..self.roots.len() {
            if self.roots[i] == id {
                idx = i as i32;
                break;
            }
        }
        if idx != -1 {
            self.roots.remove(idx as usize);
        }
    }

    pub fn get_element(&mut self, id: ElementId) -> Throws<&mut TransGuiElement> {
        self.mutated = true;
        self.modifications += 1;
        if let Some(x) = self.elements.get_mut(&id) {
            Ok(x)
        } else {
            throw!(format!("element not found:{:#?}", id));
        }
    }

    pub fn get_element_const(&self, id: ElementId) -> Throws<&TransGuiElement> {
        if let Some(x) = self.elements.get(&id) {
            Ok(x)
        } else {
            throw!(format!("element not found:{:#?}", id));
        }
    }

    pub fn get_name_id(&self, s: &str) -> ElementId {
        if let Some(id) = self.name_table.get(s) {
            *id
        } else {
            ElementId::new()
        }
    }

    pub fn remove_name(&mut self, s: &str) {
        self.name_table.remove(s);
    }

    pub fn hide_element(&mut self, id: ElementId) {
        self.modifications += 1;
        self.mutated = true;
        self.hidden.insert(id);
    }

    pub fn reveal_element(&mut self, id: ElementId) {
        self.modifications += 1;
        self.mutated = true;
        self.hidden.remove(&id);
    }

    pub fn name_element(&mut self, id: ElementId, name: impl Into<String>) {
        self.name_table.insert(name.into(), id);
    }

    fn render(&mut self, handle: &mut RaylibDrawHandle) {
        self.gui.draw_frame(handle);
    }
    pub fn remove_children(&mut self, elem: ElementId) {
        let e = self.get_element(elem).unwrap();
        match e.clone() {
            TransGuiElement::Container {
                children,
                horizontal: _,
                parent: _,
                color: _,
                upside_down: _,
            } => {
                for i in children.get() {
                    self.detach_element(*i);
                }
            }
            TransGuiElement::ScrollBox {
                scroll_amount: _,
                w: _,
                h: _,
                children,
                parent: _,
                color: _,
                upside_down: _,
            } => {
                for i in children.get() {
                    self.detach_element(*i);
                }
            }
            _ => {}
        }
    }

    fn recompute_element(&mut self, id: ElementId) {
        if self.hidden.contains(&id) {
            return;
        }
        let g = self.get_element_const(id).unwrap().clone();
        match g {
            TransGuiElement::String {
                s,
                color,
                parent: _,
            } => {
                self.gui.set_fg_color(color);
                self.gui.add_text(&*s);
            }
            TransGuiElement::Box {
                h,
                w,
                color,
                parent: _,
            } => {
                self.gui.set_fg_color(color);
                let b = self.gui.add_box(w, h);
                self.box_outputs.insert(id, b);
            }
            TransGuiElement::Button {
                color,
                on_pressed: _,
                parent: _,
                text,
            } => {
                self.gui.set_fg_color(color);
                let l = text.len();
                let w = if l > 15 { 17 } else { l as i32 + 2 };
                let h = get_string_bounds(&text, 0, 0, w).h + 3;
                let pressed = self.gui.add_button(w, h, text);
                self.button_outputs.insert(id, pressed);
            }
            TransGuiElement::Container {
                children,
                horizontal,
                parent: _,
                color,
                upside_down,
            } => {
                self.gui.set_bg_color(color);
                if horizontal {
                    self.gui.begin_div_hor();
                } else {
                    self.gui.begin_div();
                }
                if upside_down {
                    self.gui.set_upside_down();
                } else {
                    self.gui.set_rightside_up();
                }
                for i in children.get() {
                    self.recompute_element(*i);
                }
                self.gui.end_div();
            }
            TransGuiElement::ScrollBox {
                scroll_amount,
                w,
                h,
                children,
                parent: _,
                color,
                upside_down,
            } => {
                self.gui.set_bg_color(color);
                let x = self.gui.begin_scrollbox(w, h, scroll_amount);
                self.scrollbar_outputs.insert(id, x);
                if upside_down {
                    self.gui.set_upside_down();
                } else {
                    self.gui.set_rightside_up();
                }

                for i in children.get() {
                    self.recompute_element(*i);
                }
                self.gui.end_div();
            }
            TransGuiElement::BoxedGuiObject { obj: _, parent: _ } => {
                todo!()
            }
        }
    }

    fn recompute(&mut self) {
        for (id, update) in self.state_views.clone() {
            if update.as_ref().mutated(id, self) {
                self.remove_children(id);
                update.update_view(id, self);
            }
        }
        self.button_outputs.clear();
        self.scrollbar_outputs.clear();
        self.gui.begin_frame();
        let ids = self.roots.clone();
        for i in ids {
            self.recompute_element(i);
        }
    }

    pub fn update(&mut self, handle: &mut RaylibDrawHandle) {
        // println!("recomputing");
        self.recompute();
        //println!("rendering");
        self.render(handle);
        //println!("handling updates");
        self.handle_updates();
        //println!("checking if should collect");
        if self.should_collect() {
            //   println!("collecting");
            self.collect();
        }
    }

    fn handle_updates(&mut self) {
        self.mutated = false;
        let mut to_run = Vec::new();
        for (id, button) in &self.button_outputs {
            if button.take().unwrap() {
                let e = self.get_element_const(*id).unwrap();
                match e {
                    TransGuiElement::Button {
                        color: _,
                        on_pressed,
                        parent: _,
                        text: _,
                    } => {
                        to_run.push((*id, on_pressed.clone()));
                    }
                    _ => {
                        todo!()
                    }
                }
            }
        }
        let mut scroll_updates = Vec::new();
        for (id, scroll) in &self.scrollbar_outputs {
            let x = self.get_element_const(*id).unwrap();
            let Some(s) = scroll.take() else {
                continue;
            };
            match x {
                TransGuiElement::ScrollBox {
                    scroll_amount,
                    w: _,
                    h: _,
                    children: _,
                    parent: _,
                    color: _,
                    upside_down: _,
                } => {
                    if s != *scroll_amount {
                        scroll_updates.push((*id, s));
                    }
                }
                _ => {
                    todo!()
                }
            }
        }
        for (id, amount) in scroll_updates {
            let Ok(x) = self.get_element(id) else {
                continue;
            };
            match x {
                TransGuiElement::ScrollBox {
                    scroll_amount,
                    w: _,
                    h: _,
                    children: _,
                    parent: _,
                    color: _,
                    upside_down: _,
                } => {
                    *scroll_amount = amount;
                }
                _ => {
                    todo!()
                }
            }
        }
        for (id, tor) in to_run {
            let mut func = tor.lock().unwrap();
            (func)(self, id);
        }
    }

    fn collect_element(&self, element: ElementId, reachable_set: &mut HashSet<ElementId>) {
        if reachable_set.contains(&element) {
            return;
        }
        if self.get_element_const(element).is_err() {
            return;
        }
        reachable_set.insert(element);
        match self.get_element_const(element).unwrap() {
            TransGuiElement::Container {
                children,
                horizontal: _,
                parent: _,
                color: _,
                upside_down: _,
            } => {
                for i in children.get() {
                    self.collect_element(*i, reachable_set);
                }
            }
            TransGuiElement::ScrollBox {
                scroll_amount: _,
                w: _,
                h: _,
                children,
                parent: _,
                color: _,
                upside_down: _,
            } => {
                for i in children.get() {
                    self.collect_element(*i, reachable_set);
                }
            }
            _ => {}
        }
    }

    fn collect(&mut self) {
        let mut reachable_set = HashSet::new();
        for i in &self.roots {
            self.collect_element(*i, &mut reachable_set);
        }
        for id in self.name_table.values() {
            reachable_set.insert(*id);
        }
        let mut purge_list = Vec::new();
        for id in self.elements.keys() {
            if !reachable_set.contains(id) {
                purge_list.push(*id);
            }
        }
        for i in purge_list {
            self.elements.remove(&i);
            self.hidden.remove(&i);
            self.state_views.remove(&i);
        }
        self.modifications = 0;
    }

    pub fn should_collect(&self) -> bool {
        self.modifications > 20
    }

    pub fn attach_state_view(&mut self, id: ElementId, view: impl StateView) {
        let view = Arc::new(view);
        self.state_views.insert(id, view);
    }

    pub fn detach_state_view(&mut self, id: ElementId) {
        if self.state_views.remove(&id).is_some() {
            self.remove_children(id);
        }
    }

    pub fn button_output(&self, id: ElementId) -> Option<bool> {
        self.button_outputs.get(&id).and_then(|i| i.take())
    }

    pub fn scrollbar_output(&self, id: ElementId) -> Option<i32> {
        self.scrollbar_outputs.get(&id).and_then(|i| i.take())
    }

    pub fn box_output(&self, id: ElementId) -> Option<ComputedBoundary> {
        self.box_outputs.get(&id).and_then(|i| i.take())
    }
}

extern crate transir;
#[derive(Clone)]
pub enum TransIr {
    String {
        s: String,
        color: Option<Color>,
        name: Option<String>,
    },
    Box {
        h: i32,
        w: i32,
        color: Option<Color>,
        name: Option<String>,
    },
    Button {
        color: Option<Color>,
        on_pressed: Arc<Mutex<GuiUpdateFn>>,
        text: String,
        name: Option<String>,
    },
    Container {
        children: Vec<TransIr>,
        horizontal: bool,
        color: Option<Color>,
        upside_down: bool,
        name: Option<String>,
    },
    ScrollBox {
        w: i32,
        h: i32,
        children: Vec<TransIr>,
        color: Option<Color>,
        upside_down: bool,
        name: Option<String>,
    },
}
impl TransIr {
    pub fn add_to_gui(self, gui: &mut TransGui) -> ElementId {
        match self {
            TransIr::String { s, color, name } => {
                if let Some(c) = color {
                    gui.fg_color = c;
                };
                let e = gui.new_text(s);
                if let Some(name) = name {
                    gui.name_element(e, name);
                };
                e
            }
            TransIr::Box { h, w, color, name } => {
                if let Some(c) = color {
                    gui.fg_color = c;
                };
                let e = gui.new_box(h, w);
                if let Some(name) = name {
                    gui.name_element(e, name);
                };
                e
            }
            TransIr::Button {
                color,
                on_pressed,
                text,
                name,
            } => {
                if let Some(c) = color {
                    gui.fg_color = c;
                };
                let e = gui.new_button(|_, _| {}, text);
                let elem = gui.get_element(e).unwrap();
                let op = on_pressed;
                match elem {
                    TransGuiElement::Button {
                        color: _,
                        on_pressed,
                        parent: _,
                        text: _,
                    } => {
                        *on_pressed = op;
                    }
                    _ => {
                        todo!()
                    }
                };
                if let Some(name) = name {
                    gui.name_element(e, name);
                };
                e
            }
            TransIr::Container {
                children,
                horizontal,
                color,
                upside_down,
                name,
            } => {
                if let Some(c) = color {
                    gui.fg_color = c;
                }
                let elem = if horizontal {
                    gui.new_horizontal_section()
                } else if upside_down {
                    gui.new_section_upside_down()
                } else {
                    gui.new_section()
                };
                if let Some(name) = name {
                    gui.name_element(elem, name);
                }
                let mut child_vec = Vec::new();
                for i in children {
                    if let Some(c) = color {
                        gui.fg_color = c;
                    }
                    child_vec.push(i.add_to_gui(gui));
                }
                for i in child_vec {
                    gui.attach_to_element(i, elem);
                }
                elem
            }
            TransIr::ScrollBox {
                w,
                h,
                children,
                color,
                upside_down,
                name,
            } => {
                if let Some(c) = color {
                    gui.fg_color = c;
                }
                let elem = if upside_down {
                    gui.new_scroll_box_upside_down(w, h)
                } else {
                    gui.new_scroll_box(w, h)
                };
                if let Some(name) = name {
                    gui.name_element(elem, name);
                }
                let mut child_vec = Vec::new();
                for i in children {
                    if let Some(c) = color {
                        gui.fg_color = c;
                    }
                    child_vec.push(i.add_to_gui(gui));
                }
                for i in child_vec {
                    gui.attach_to_element(i, elem);
                }
                elem
            }
        }
    }
    pub fn to_gui(list: Vec<Self>) -> TransGui {
        let mut out = TransGui::new();
        for i in list {
            let elem = i.add_to_gui(&mut out);
            out.attach_to_doc(elem);
        }
        out
    }
}

pub struct ListView<T: Clone + 'static, U: Fn(&SharedList<T>, ElementId, &mut TransGui) + 'static> {
    inner: SharedList<T>,
    on_update: U,
}
impl<T: Clone + 'static, U: Fn(&SharedList<T>, ElementId, &mut TransGui) + 'static> StateView
    for ListView<T, U>
{
    fn mutated(&self, _id: ElementId, _gui: &TransGui) -> bool {
        self.inner.consume_mutation()
    }

    fn update_view(&self, id: ElementId, gui: &mut TransGui) {
        (self.on_update)(&self.inner, id, gui);
    }
}

impl<T: Clone + 'static, U: Fn(&SharedList<T>, ElementId, &mut TransGui) + 'static> ListView<T, U> {
    pub fn new(list: &SharedList<T>, on_update: U) -> Self {
        Self {
            inner: list.clone(),
            on_update,
        }
    }
}
pub type TransOutput<T> = TGuiOutput<T>;
