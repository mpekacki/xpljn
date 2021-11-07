extern crate jsonpath;
extern crate serde_json;
extern crate sxd_document;
extern crate sxd_xpath;

use jsonpath::Selector;
use regex::Regex;
use serde_json::Value;
use std::fs::File;
use std::io::{self, Write};
use std::{env, fs, path::PathBuf};
use sxd_document::parser;
use sxd_xpath::evaluate_xpath;
use tempfile::tempdir;

pub trait ExpressionReplacer {
    fn replace(&self, contents: &String) -> String {
        let re = Regex::new(self.get_regex_string()).unwrap();
        let mut replaced: String = contents.to_string();
        for cap in re.captures_iter(&contents) {
            let res_file = &cap[1];
            let expression = &cap[2];
            let res_contents = fs::read_to_string(res_file).unwrap_or_else(|err| {
                panic!("Error reading file: {:?}, {:?}", err, res_file);
            });
            let val = self.search(&res_contents, expression);
            replaced = re.replace(&replaced, val).to_string();
        }
        return replaced;
    }
    fn get_regex_string(&self) -> &str;
    fn search(&self, file_content: &str, expression: &str) -> String;
}

pub struct XmlReplacer;

impl ExpressionReplacer for XmlReplacer {
    fn get_regex_string(&self) -> &str {
        r#"\{([\w/\\\.:-~]+)#([ =\w/\[\]"'.:@]+)\}"#
    }
    fn search(&self, file_content: &str, expression: &str) -> String {
        let package = parser::parse(&file_content).unwrap();
        let document = package.as_document();
        let val = evaluate_xpath(&document, expression).unwrap().string();
        return val;
    }
}

pub struct JsonReplacer;

impl ExpressionReplacer for JsonReplacer {
    fn get_regex_string(&self) -> &str {
        r#"\{([\w/\\\.:-~]+)#([$@*.\[\]():?<>!=~\w' ]+)\}"#
    }
    fn search(&self, file_content: &str, expression: &str) -> String {
        let json: Value = serde_json::from_str(&file_content).unwrap();
        let selector = Selector::new(&expression).unwrap();
        let matches: Vec<&str> = selector.find(&json).map(|t| t.as_str().unwrap()).collect();
        return matches[0].to_string();
    }
}

pub struct Config {
    extension: String,
    dir: PathBuf,
    replacers: Vec<Box<dyn ExpressionReplacer>>,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &str> {
        let mut dir = env::current_dir().unwrap();
        let mut extension = ".template".to_string();
        if args.len() > 1 {
            dir = PathBuf::from(&args[1]);
        }
        if args.len() > 2 {
            extension = args[2].to_string();
        }
        let replacers: Vec<Box<dyn ExpressionReplacer>> =
            vec![Box::new(XmlReplacer), Box::new(JsonReplacer)];

        Ok(Config {
            extension,
            dir,
            replacers,
        })
    }
}

pub fn run(config: Config) {
    for entry in fs::read_dir(config.dir).unwrap() {
        let entry = entry.unwrap();
        let mut file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.ends_with(&config.extension) {
            let mut contents = fs::read_to_string(entry.file_name()).unwrap().clone();
            for replacer in config.replacers.iter() {
                contents = replacer.replace(&contents);
            }
            file_name.truncate(file_name.len() - config.extension.len());
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
        writeln!(file, r#"<?xml version="1.0" encoding="UTF-8" ?>"#).unwrap();
        writeln!(file, r#"<Resources>"#).unwrap();
        writeln!(file, r#"  <Strings>"#).unwrap();
        writeln!(file, r#"    <Hello>Hi!</Hello>"#).unwrap();
        writeln!(file, r#"    <Bye>Byebye!</Bye>"#).unwrap();
        writeln!(file, r#"  </Strings>"#).unwrap();
        writeln!(file, r#"</Resources>"#).unwrap();
        let file_path_str = file_path.into_os_string().into_string().unwrap();
        let xpath = "/Resources/Strings/Bye";
        let contents = format!("let label = '{{{}#{}}}'", file_path_str, xpath);
        let replaced = XmlReplacer.replace(&contents);
        assert_eq!(replaced, "let label = 'Byebye!'");
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
        let file_path_str = file_path.into_os_string().into_string().unwrap();
        let jsonpath = "$.Bye";
        let contents = format!("let label = '{{{}#{}}}'", file_path_str, jsonpath);
        let replaced = JsonReplacer.replace(&contents);
        assert_eq!(replaced, "let label = 'Byebye!'");
    }

    #[test]
    fn test_json_replacer_with_array() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, r#"{{"#).unwrap();
        writeln!(file, r#"  "Hello": "Hi!", "#).unwrap();
        writeln!(file, r#"  "Bye": ["Byebye!", "Ja ne!"]"#).unwrap();
        writeln!(file, r#"}}"#).unwrap();
        let file_path_str = file_path.into_os_string().into_string().unwrap();
        let jsonpath = "$.Bye[1]";
        let contents = format!("let label = '{{{}#{}}}'", file_path_str, jsonpath);
        let replaced = JsonReplacer.replace(&contents);
        assert_eq!(replaced, "let label = 'Ja ne!'");
    }

    #[test]
    fn test_xml_replacer_with_array() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.xml");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, r#"<?xml version="1.0" encoding="UTF-8" ?>"#).unwrap();
        writeln!(file, r#"<Resources>"#).unwrap();
        writeln!(file, r#"  <Strings>"#).unwrap();
        writeln!(file, r#"    <Hello>Hi!</Hello>"#).unwrap();
        writeln!(file, r#"    <Bye>Byebye!</Bye>"#).unwrap();
        writeln!(file, r#"  </Strings>"#).unwrap();
        writeln!(file, r#"  <Strings>"#).unwrap();
        writeln!(file, r#"    <Hello>Ossu!</Hello>"#).unwrap();
        writeln!(file, r#"    <Bye>Ja ne!</Bye>"#).unwrap();
        writeln!(file, r#"  </Strings>"#).unwrap();
        writeln!(file, r#"</Resources>"#).unwrap();
        let file_path_str = file_path.into_os_string().into_string().unwrap();
        let xpath = "/Resources/Strings[2]/Bye";
        let contents = format!("let label = '{{{}#{}}}'", file_path_str, xpath);
        let replaced = XmlReplacer.replace(&contents);
        assert_eq!(replaced, "let label = 'Ja ne!'");
    }

    #[test]
    fn test_json_replacer_with_deeply_nested_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, r#"{{"#).unwrap();
        writeln!(file, r#"  "Hello": "Hi!", "#).unwrap();
        writeln!(file, r#"  "Bye": {{"#).unwrap();
        writeln!(file, r#"    "Hello": "Byebye!", "#).unwrap();
        writeln!(file, r#"    "Bye": {{"#).unwrap();
        writeln!(file, r#"      "Hello": "Ja ne!""#).unwrap();
        writeln!(file, r#"    }}"#).unwrap();
        writeln!(file, r#"  }}"#).unwrap();
        writeln!(file, r#"}}"#).unwrap();
        let file_path_str = file_path.into_os_string().into_string().unwrap();
        let jsonpath = "$.Bye.Bye.Hello";
        let contents = format!("let label = '{{{}#{}}}'", file_path_str, jsonpath);
        let replaced = JsonReplacer.replace(&contents);
        assert_eq!(replaced, "let label = 'Ja ne!'");
    }

    #[test]
    fn test_xml_replacer_with_attribute_selector() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.xml");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, r#"<?xml version="1.0" encoding="UTF-8" ?>"#).unwrap();
        writeln!(file, r#"<Resources>"#).unwrap();
        writeln!(file, r#"  <Strings lang="en">"#).unwrap();
        writeln!(file, r#"    <Hello>Hi!</Hello>"#).unwrap();
        writeln!(file, r#"    <Bye>Byebye!</Bye>"#).unwrap();
        writeln!(file, r#"  </Strings>"#).unwrap();
        writeln!(file, r#"  <Strings lang="jp">"#).unwrap();
        writeln!(file, r#"    <Hello>Ossu!</Hello>"#).unwrap();
        writeln!(file, r#"    <Bye>Ja ne!</Bye>"#).unwrap();
        writeln!(file, r#"  </Strings>"#).unwrap();
        writeln!(file, r#"</Resources>"#).unwrap();
        let file_path_str = file_path.into_os_string().into_string().unwrap();
        let xpath = "/Resources/Strings[@lang='jp']/Bye";
        let contents = format!("let label = '{{{}#{}}}'", file_path_str, xpath);
        let replaced = XmlReplacer.replace(&contents);
        assert_eq!(replaced, "let label = 'Ja ne!'");
    }

    #[test]
    fn test_xml_replacer_with_multiple_matches() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.xml");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, r#"<?xml version="1.0" encoding="UTF-8" ?>"#).unwrap();
        writeln!(file, r#"<Resources>"#).unwrap();
        writeln!(file, r#"  <Strings lang="en">"#).unwrap();
        writeln!(file, r#"    <Hello>Hi!</Hello>"#).unwrap();
        writeln!(file, r#"    <Bye>Byebye!</Bye>"#).unwrap();
        writeln!(file, r#"  </Strings>"#).unwrap();
        writeln!(file, r#"  <Strings lang="jp">"#).unwrap();
        writeln!(file, r#"    <Hello>Ossu!</Hello>"#).unwrap();
        writeln!(file, r#"    <Bye>Ja ne!</Bye>"#).unwrap();
        writeln!(file, r#"  </Strings>"#).unwrap();
        writeln!(file, r#"</Resources>"#).unwrap();
        let file_path_str = file_path.into_os_string().into_string().unwrap();
        let xpath = "/Resources/Strings/Bye";
        let contents = format!("let label = '{{{}#{}}}'", file_path_str, xpath);
        let replaced = XmlReplacer.replace(&contents);
        assert_eq!(replaced, "let label = 'Byebye!'", "Should use first match");
    }
}
