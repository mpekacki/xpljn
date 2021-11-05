extern crate jsonpath;
extern crate serde_json;
extern crate sxd_document;
extern crate sxd_xpath;

use jsonpath::Selector;
use regex::Regex;
use serde_json::Value;
use std::{env, fs, path::PathBuf};
use sxd_document::parser;
use sxd_xpath::evaluate_xpath;
use tempfile::tempdir;
use std::fs::File;
use std::io::{self, Write};

pub trait ExpressionReplacer {
    fn replace(contents: &String) -> String;
}

pub struct XmlReplacer;

impl ExpressionReplacer for XmlReplacer {
    fn replace(contents: &String) -> String {
        let re_xml = Regex::new(r#"\{([\w/\\\.:-~]+)#([ =\w/\[\]":]+)\}"#).unwrap();
        let mut replaced: String = contents.to_string();
        for cap in re_xml.captures_iter(&contents) {
            let res_file = &cap[1];
            let xpath = &cap[2];
            println!("Res file: {:?}, XPath: {:?}", res_file, xpath);
            let res_contents = fs::read_to_string(res_file).unwrap_or_else(|err| {
                panic!("Error reading file: {:?}, {:?}", err, res_file);
            });
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
        let re_json = Regex::new(r#"\{([\w/\\\.:-~]+)#([$@*.\[\]():?<>!=~\w' ]+)\}"#).unwrap();
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

pub struct Config {
    extension: String,
    dir: PathBuf,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &str> {
        let dir = env::current_dir().expect("Problem with getting current dir");
        let extension = String::from(".template");

        Ok(Config { extension, dir })
    }
}

pub fn run(config: Config) {
    for entry in fs::read_dir(config.dir).unwrap() {
        let entry = entry.unwrap();
        let mut file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.ends_with(&config.extension) {
            let mut contents = fs::read_to_string(entry.file_name()).unwrap().clone();
            println!("Contents: {:?}", contents);
            contents = XmlReplacer::replace(&contents);
            contents = JsonReplacer::replace(&contents);
            println!("Replaced: {:?}", contents);
            file_name.truncate(file_name.len() - config.extension.len());
            println!("New name: {:?}", file_name);
            fs::write(file_name, contents).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
   use super::*;

    #[test]
    fn test_xml_replacer() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.xml");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "<?xml version=\"1.0\" encoding=\"UTF-8\" ?><Resources><Strings><Hello>Hi!</Hello><Bye>Byebye!</Bye></Strings></Resources>").unwrap();
        let contents = format!("{}{}{}", r#"{"#.to_owned(), file_path.into_os_string().into_string().unwrap(), r#"#/Resources/Strings/Bye}"#);
        let replaced = XmlReplacer::replace(&contents);
        assert_eq!(replaced, "Byebye!");
    }

    #[test]
    fn test_json_replacer() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, r#"{{"#).unwrap();
        writeln!(file, r#"  "Hello": "Hi!", "#).unwrap();
        writeln!(file, r#"  "Bye": "Byebye!""#).unwrap();
        writeln!(file, r#"}}"#).unwrap();
        let contents = format!("{}{}{}", r#"{"#.to_owned(), file_path.into_os_string().into_string().unwrap(), r#"#$.Bye}"#);
        let replaced = JsonReplacer::replace(&contents);
        assert_eq!(replaced, "Byebye!");
    }

    #[test]
    fn test_json_replacer_with_array() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, r#"{{"#).unwrap();
        writeln!(file, r#"  "Hello": "Hi!", "#).unwrap();
        writeln!(file, r#"  "Bye": ["Byebye!", "Byebye!"]"#).unwrap();
        writeln!(file, r#"}}"#).unwrap();
        let contents = format!("{}{}{}", r#"{"#.to_owned(), file_path.into_os_string().into_string().unwrap(), r#"#$.Bye[1]}"#);
        let replaced = JsonReplacer::replace(&contents);
        assert_eq!(replaced, "Byebye!");
    }
}