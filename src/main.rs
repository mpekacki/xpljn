extern crate jsonpath;
extern crate serde_json;
extern crate sxd_document;
extern crate sxd_xpath;

use jsonpath::Selector;
use regex::Regex;
use serde_json::Value;
use std::{env, fs};
use sxd_document::parser;
use sxd_xpath::evaluate_xpath;

pub trait ExpressionReplacer {
    fn replace(contents: &String) -> String;
}

pub struct XmlReplacer;

impl ExpressionReplacer for XmlReplacer {
    fn replace(contents: &String) -> String {
        let re_xml = Regex::new(r#"\{([\w/\\\.:-]+)#([ =\w/\[\]":]+)\}"#).unwrap();
        let mut replaced: String = contents.to_string();
        for cap in re_xml.captures_iter(&contents) {
            let res_file = &cap[1];
            let xpath = &cap[2];
            println!("Res file: {:?}, XPath: {:?}", res_file, xpath);
            let res_contents = fs::read_to_string(res_file).unwrap();
            println!("Res contents: {:?}", res_contents);
            let package = parser::parse(&res_contents).unwrap();
            let document = package.as_document();
            println!("The document: {:?}", document.root().children());
            let val = evaluate_xpath(&document, xpath).unwrap().string();
            println!("The value is: {:?}", val);
            replaced = re_xml.replace(&replaced, val).to_string();
            println!("Replaced: {:?}", contents);
        }
        return replaced;
    }
}

pub struct JsonReplacer;

impl ExpressionReplacer for JsonReplacer {
    fn replace(contents: &String) -> String {
        let re_json = Regex::new(r#"\{([\w/\\\.:-]+)#([$@*.\[\]():?<>!=~\w' ]+)\}"#).unwrap();
        let mut replaced: String = contents.to_string();
        for cap in re_json.captures_iter(&contents) {
            let res_file = &cap[1];
            let jsonpath = &cap[2];
            println!("Res file: {:?}, JSONPath: {:?}", res_file, jsonpath);
            let res_contents = fs::read_to_string(res_file).unwrap();
            println!("Res contents: {:?}", res_contents);
            let json: Value = serde_json::from_str(&res_contents).unwrap();
            let selector = Selector::new(&jsonpath).unwrap();
            let matches: Vec<&str> = selector.find(&json).map(|t| t.as_str().unwrap()).collect();
            replaced = re_json.replace(&replaced, matches[0]).to_string();
            println!("Replaced: {:?}", contents);
        }
        return replaced;
    }
}

fn main() {
    let current_dir = env::current_dir().unwrap();
    println!("Files in current dir ({:?}):", current_dir);
    let extension = ".template";

    for entry in fs::read_dir(current_dir).unwrap() {
        let entry = entry.unwrap();
        let mut file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.ends_with(extension) {
            let mut contents = fs::read_to_string(entry.file_name()).unwrap().clone();
            println!("Contents: {:?}", contents);
            contents = XmlReplacer::replace(&contents);
            contents = JsonReplacer::replace(&contents);
            println!("Replaced: {:?}", contents);
            file_name.truncate(file_name.len() - extension.len());
            println!("New name: {:?}", file_name);
            fs::write(file_name, contents).unwrap();
        }
    }
}
