extern crate sxd_document;
extern crate sxd_xpath;

use std::{env, fs, io::Write, path::Path};
use regex::Regex;
use sxd_document::parser;
use sxd_xpath::{evaluate_xpath};

fn main() {
    let current_dir = env::current_dir().unwrap();
    println!("Files in current dir ({:?}):", current_dir);
    let extension = ".template";
    let re_xml = Regex::new(r#"\{([\w/\\\.:-]+)#([ =\w/\[\]":]+)\}"#).unwrap();

    for entry in fs::read_dir(current_dir).unwrap() {
        let entry = entry.unwrap();
        let mut file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.ends_with(extension) {
            let contents = fs::read_to_string(entry.file_name()).unwrap();
            let mut replaced = contents.clone();
            let is_match = re_xml.is_match(&contents);
            println!("Contents: {:?}, is match: {:?}", contents, is_match);
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
                println!("Replaced: {:?}", replaced);
            }
            file_name.truncate(file_name.len() - extension.len());
            println!("New name: {:?}", file_name);
            fs::write(file_name, replaced).unwrap();
        }
    }
}
