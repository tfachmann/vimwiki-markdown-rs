use kuchiki::traits::*;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use std::collections::HashMap;

struct VarStore {
    map: HashMap<String, String>,
}

lazy_static! {
    static ref RE_DEF: Regex = Regex::new(r"<'''(?P<data>(.|\n)*)'''>").unwrap();
    static ref RE_DEF_SINGLE: Regex = Regex::new(r"(?P<key>\S*?)\{(?P<value>[^}]*?)\}").unwrap();
    static ref RE_VAR: Regex =
        Regex::new(r"'\{(?P<before>.*?)\$(?P<var>\S+?)(?P<after>(\s.*?\}|\}))'").unwrap();
    static ref RE_CMD: Regex =
        Regex::new(r"'\{(?P<element>\S+)\s+(?P<type>\S+)\s+(?P<data>.*?)\}'").unwrap();
}

impl VarStore {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    fn parse_variables(&mut self, input: &str) {
        // parse defined commands
        let mut caps_it = RE_DEF.captures_iter(&input);
        let capture = caps_it.next();
        match capture {
            Some(c) => {
                RE_DEF_SINGLE
                    .captures_iter(&c["data"])
                    .into_iter()
                    .for_each(|e| {
                        self.map.insert(e["key"].to_owned(), e["value"].to_owned());
                    });
            }
            None => (),
        }
    }

    fn clear_variables(&self, text: &str) -> String {
        RE_DEF.replace_all(&text, "").to_string()
    }

    fn replace_variables(&self, text: &str) -> String {
        // Checks whether variables were used and replaces them
        // TODO: do this recursively, until all occurences are fixed
        RE_VAR
            .replace_all(&text, |caps: &Captures| {
                let val = match self.map.get(&caps["var"]) {
                    Some(value) => value,
                    None => panic!("Cannot find variable `{}`", &caps["var"]),
                };
                // due to the nature of the regex, the last } will always be included at the end
                let before = &caps["before"];
                let after = &caps["after"][0..&caps["after"].len() - 1];
                format!("'{{{}{}{}}}'", before, val, after)
            })
            .to_string()
    }

    /// Parses an input (content of markdown file) for commands and returns a cleaned text
    pub fn parse(&mut self, input: &str) -> String {
        self.parse_variables(input);
        let cleaned = self.clear_variables(input);
        self.replace_variables(&cleaned)
    }
}

pub fn preprocess_variables(markdown: &str) -> String {
    let mut var_store = VarStore::new();
    var_store.parse(&markdown)
}

pub fn apply_commands(html: &str) -> String {
    let mut change_parents = vec![];

    let document = kuchiki::parse_html().one(html.clone());
    document.descendants().for_each(|node| {
        if let Some(text) = node.as_text() {
            if let Some(capture) = RE_CMD.captures_iter(&text.borrow()).next() {
                let element_type = &capture["element"];
                let html_attribute = match &capture["type"] {
                    "s" | "st" | "sty" | "styl" | "style" => "style",
                    _ => panic!("HTML attribute `{}` unknown", &capture["type"]),
                };
                let data = &capture["data"];
                match element_type {
                    "p" | "pa" | "par" | "pare" | "paren" | "parent" => {
                        if let Some(parent) = node.parent() {
                            if let Some(element_data) = parent.as_element() {
                                let mut att = element_data.attributes.borrow_mut();
                                att.insert(html_attribute, data.to_string());
                            }
                            change_parents.push((parent, data.to_owned()));
                        }
                    }
                    _ => panic!("Element type `{}` unknown", element_type),
                };
            }
        };
    });

    // delte all commands
    RE_CMD.replace_all(&document.to_string(), "").to_string()
}
