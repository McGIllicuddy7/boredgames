use std::{collections::HashMap, sync::Arc};



use super::draw::Image;
#[derive(Clone, Copy)]
pub enum WidgetType{
    Div,
    Text, 
    Button, 
    Image,
}

#[derive(Clone, Copy)]
pub enum Orientation{
    Horizontal, 
    Vertical, 
}   

pub struct Widget{ 

}

pub struct Gui{
    pub image:Image, 
    pub widget_stack:Vec<Widget>,
    pub completed:Vec<Arc<Widget>>, 
    pub previous_table:HashMap<String, Arc<Widget>>, 
    pub current_table:HashMap<String, Arc<Widget>>, 
    pub images:HashMap<String, Image>,
}