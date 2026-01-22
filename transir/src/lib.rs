use std::collections::HashMap;

use proc_macro::{TokenStream, TokenTree};
#[derive(Clone)]
enum LispLike<T> {
    Value { v: T },
    List { v: Vec<LispLike<T>> },
}
impl LispLike<TokenTree> {
    fn flatten(&self) -> String {
        match self {
            Self::Value { v } => v.to_string(),
            Self::List { v } => {
                let mut out = String::new();
                for i in v {
                    if out != "" {
                        out += " ";
                    }
                    out += &i.flatten();
                }
                out
            }
        }
    }
}
fn lisp_like_collect(stream: &mut impl Iterator<Item = TokenTree>) -> Vec<LispLike<TokenTree>> {
    let mut out = Vec::new();
    loop {
        let Some(x) = stream.next() else {
            break;
        };
        match x.clone() {
            TokenTree::Group(group) => match group.delimiter() {
                proc_macro::Delimiter::Parenthesis => {
                    let stream2 = group.stream();
                    let list = lisp_like_collect(&mut stream2.into_iter());
                    out.push(LispLike::List { v: list });
                }
                _ => {
                    out.push(LispLike::Value { v: x });
                }
            },
            _ => {
                out.push(LispLike::Value { v: x });
            }
        }
    }
    out
}

fn parse_item(item: LispLike<TokenTree>) -> String {
    match item {
        LispLike::Value { v } => {
            format!("{}", v.to_string())
        }
        LispLike::List { v } => {
            if v.is_empty() {
                format!("")
            } else {
                let list = v;
                let first = list[0].clone();
                match first {
                    LispLike::Value { v } => {
                        let s = v.to_string();
                        match s.as_str() {
                            "div" => {
                                let mut hs = HashMap::new();
                                let mut iter = list.into_iter();
                                let _ = iter.next();
                                let out;
                                loop {
                                    let x = iter.next().unwrap();
                                    let s = x.flatten();
                                    if s == "reverse" {
                                        hs.insert("reverse", None);
                                    } else if s == "color" {
                                        let n = iter.next().unwrap();
                                        hs.insert("color", Some(n.flatten()));
                                    } else if s == "horizontal" {
                                        hs.insert("horizontal", None);
                                    } else if s == "name" {
                                        let eq = iter.next().unwrap();
                                        if eq.flatten() != "=" {
                                            todo!();
                                        }
                                        let name = iter.next().unwrap();
                                        hs.insert("name", Some(name.flatten() + ".to_string()"));
                                    } else {
                                        out = x;
                                        break;
                                    }
                                }
                                let mut string = String::new();
                                match out {
                                    LispLike::List { v } => {
                                        for i in v {
                                            if string != "" {
                                                string += ",";
                                            }
                                            string += &parse_item(i);
                                        }
                                    }
                                    LispLike::Value { v: _ } => {
                                        todo!()
                                    }
                                }
                                let color = if hs.contains_key("color") {
                                    hs["color"].clone().unwrap_or("None".to_string())
                                } else {
                                    "None".to_string()
                                };
                                let horizontal = if hs.contains_key("horizontal") {
                                    "true"
                                } else {
                                    "false"
                                };
                                let upside_down = if hs.contains_key("reverse") {
                                    "true"
                                } else {
                                    "false"
                                };
                                let name = if hs.contains_key("name") {
                                    hs["name"].clone().unwrap_or("None".to_string())
                                } else {
                                    "None".to_string()
                                };
                                format!("TransIr::Container{{
                                    children:vec![{}], color:{color}.into(), horizontal:{horizontal}.into(), upside_down:{upside_down}, name:{name}.into()
                                }}", string)
                            }
                            "scrollbox" => {
                                let mut hs = HashMap::new();
                                let mut iter = list.into_iter();
                                let _ = iter.next();
                                let out;
                                loop {
                                    let x = iter.next().unwrap();
                                    let s = x.flatten();
                                    if s == "reverse" {
                                        hs.insert("reverse", None);
                                    } else if s == "color" {
                                        let n = iter.next().unwrap();
                                        hs.insert("color", Some(parse_item(n)));
                                    } else if s == "name" {
                                        let eq = iter.next().unwrap();
                                        if eq.flatten() != "=" {
                                            todo!();
                                        }
                                        let name = iter.next().unwrap();
                                        hs.insert("name", Some(parse_item(name) + ".to_string()"));
                                    } else if s == "w" {
                                        let w = iter.next().unwrap();
                                        hs.insert("h", Some(parse_item(w)));
                                    } else if s == "h" {
                                        let h = iter.next().unwrap();
                                        hs.insert("h", Some(parse_item(h)));
                                    } else {
                                        out = x;
                                        break;
                                    }
                                }
                                let mut string = String::new();
                                match out {
                                    LispLike::List { v } => {
                                        for i in v {
                                            if string != "" {
                                                string += ",";
                                            }
                                            string += &parse_item(i);
                                        }
                                    }
                                    LispLike::Value { v: _ } => {
                                        todo!()
                                    }
                                }
                                let color = if hs.contains_key("color") {
                                    hs["color"].clone().unwrap_or("None".to_string())
                                } else {
                                    "None".to_string()
                                };
                                let upside_down = if hs.contains_key("reverse") {
                                    "true"
                                } else {
                                    "false"
                                };
                                let name = if hs.contains_key("name") {
                                    hs["name"].clone().unwrap_or("None".to_string())
                                } else {
                                    "None".to_string()
                                };
                                let h = if hs.contains_key("h") {
                                    hs["h"].clone().unwrap_or("10".to_string())
                                } else {
                                    "10".to_string()
                                };
                                let w = if hs.contains_key("w") {
                                    hs["w"].clone().unwrap_or("10".to_string())
                                } else {
                                    "10".to_string()
                                };
                                format!("TransIr::ScrollBox{{
                                    children:vec![{}], color:{color}.into(),upside_down:{upside_down}.into(), name:{name}.into(), w:{w}, h:{h}
                                }}", string)
                            }
                            "text" => {
                                let mut hs = HashMap::new();
                                let mut iter = list.into_iter();
                                let _ = iter.next();
                                let out;
                                loop {
                                    let x = iter.next().unwrap();
                                    let s = x.flatten();
                                    if s == "color" {
                                        let n = iter.next().unwrap();
                                        hs.insert("color", Some(parse_item(n)));
                                    } else if s == "name" {
                                        let eq = iter.next().unwrap();
                                        if eq.flatten() != "=" {
                                            todo!();
                                        }
                                        let name = iter.next().unwrap();
                                        hs.insert("name", Some(parse_item(name) + ".to_string()"));
                                    } else {
                                        out = x;
                                        break;
                                    }
                                }
                                let string = out.flatten();
                                let color = if hs.contains_key("color") {
                                    hs["color"].clone().unwrap_or("None".to_string())
                                } else {
                                    "None".to_string()
                                };
                                let name = if hs.contains_key("name") {
                                    hs["name"].clone().unwrap_or("None".to_string())
                                } else {
                                    "None".to_string()
                                };
                                format!(
                                    "TransIr::String{{
                                    name:{name}.into(), color:{color}.into(), s:{}.to_string()
                                }}",
                                    string
                                )
                            }
                            "button" => {
                                let mut hs = HashMap::new();
                                let mut iter = list.into_iter();
                                let _ = iter.next();
                                let out;
                                loop {
                                    let x = iter.next().unwrap();
                                    let s = x.flatten();
                                    if s == "color" {
                                        let n = iter.next().unwrap();
                                        hs.insert("color", Some(parse_item(n)));
                                    } else if s == "name" {
                                        let eq = iter.next().unwrap();
                                        if eq.flatten() != "=" {
                                            todo!();
                                        }
                                        let name = iter.next().unwrap();
                                        hs.insert("name", Some(parse_item(name)));
                                    } else {
                                        out = x;
                                        break;
                                    }
                                }
                                let string = out.flatten() + ".to_string()";
                                let to_call = iter.next().unwrap().flatten();
                                let color = if hs.contains_key("color") {
                                    hs["color"].clone().unwrap_or("None".to_string())
                                } else {
                                    "None".to_string()
                                };
                                let name = if hs.contains_key("name") {
                                    hs["name"].clone().unwrap_or("None".to_string())
                                } else {
                                    "None".to_string()
                                };
                                format!("TransIr::Button{{
                                    name:{name}, color:{color}, text:{}, on_pressed:Arc::new(Mutex::new(({})))
                                }}", string, to_call)
                            }
                            "box" => {
                                let mut hs = HashMap::new();
                                let mut iter = list.into_iter();
                                let _ = iter.next();
                                loop {
                                    let Some(x) = iter.next() else {
                                        break;
                                    };
                                    let s = x.flatten();
                                    if s == "color" {
                                        let n = iter.next().unwrap();
                                        hs.insert("color", Some(parse_item(n)));
                                    } else if s == "name" {
                                        let eq = iter.next().unwrap();
                                        if eq.flatten() != "=" {
                                            todo!();
                                        }
                                        let name = iter.next().unwrap();
                                        hs.insert("name", Some(parse_item(name) + ".to_string()"));
                                    } else if s == "w" {
                                        let w = iter.next().unwrap();
                                        hs.insert("w", Some(parse_item(w)));
                                    } else if s == "h" {
                                        let h = iter.next().unwrap();
                                        hs.insert("h", Some(parse_item(h)));
                                    } else {
                                        break;
                                    }
                                }
                                let color = if hs.contains_key("color") {
                                    hs["color"].clone().unwrap_or("None".to_string())
                                } else {
                                    "None".to_string()
                                };
                                let name = if hs.contains_key("name") {
                                    hs["name"].clone().unwrap_or("None".to_string())
                                } else {
                                    "None".to_string()
                                };
                                let h = if hs.contains_key("h") {
                                    hs["h"].clone().unwrap_or("10".to_string())
                                } else {
                                    "10".to_string()
                                };
                                let w = if hs.contains_key("w") {
                                    hs["w"].clone().unwrap_or("10".to_string())
                                } else {
                                    "10".to_string()
                                };
                                format!(
                                    "TransIr::Box{{h:{h}, w:{w}, color:{color}, name:{name}.into()}}"
                                )
                            }
                            _ => v.to_string(),
                        }
                    }
                    LispLike::List { v } => {
                        let mut out = String::new();
                        for i in v {
                            if out != "" {
                                out += " ";
                            }
                            out += &i.flatten();
                        }
                        out
                    }
                }
            }
        }
    }
}
#[proc_macro]
pub fn trans(stream: TokenStream) -> TokenStream {
    let out: Vec<LispLike<TokenTree>> = lisp_like_collect(&mut stream.into_iter());
    let mut strings = Vec::new();
    for i in out {
        strings.push(parse_item(i));
    }
    let mut outs = String::new();
    for i in strings {
        if outs != "" {
            outs += ",";
        }
        outs += &i;
    }
    format!("vec![{}]", outs).parse().unwrap()
}
