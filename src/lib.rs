extern crate jsonpath;
extern crate serde_json;
extern crate sxd_document;
extern crate sxd_xpath;

use jsonpath::Selector;
use regex::Regex;
use serde_json::Value;
use std::io::{Read, Write};
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
        let mut file_path = entry.path().to_str().unwrap().to_string();
        if file_path.ends_with(&config.extension) {
            let mut contents = fs::read_to_string(&file_path).unwrap_or_else(|err| {
                panic!("Error reading file: {:?}, {:?}", err, entry.file_name());
            });
            for replacer in config.replacers.iter() {
                contents = replacer.replace(&contents);
            }
            file_path.truncate(file_path.len() - config.extension.len());
            fs::write(file_path, contents).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_replacer(
        res_file_lines: Vec<&str>,
        expression: &str,
        expected: &str,
        replacer: &dyn ExpressionReplacer,
    ) {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");
        let mut file = fs::File::create(&file_path).unwrap();
        for line in res_file_lines {
            writeln!(file, "{}", line).unwrap();
        }
        let file_path_str = file_path.into_os_string().into_string().unwrap();
        let contents = format!("let label = '{{{}#{}}}'", file_path_str, expression);
        let replaced = replacer.replace(&contents);
        assert_eq!(replaced, "let label = '".to_string() + expected + "'");
    }

    #[test]
    fn test_xml_replacer_with_xpath() {
        test_replacer(
            vec![
                r#"<?xml version="1.0" encoding="UTF-8" ?>"#,
                r#"<Resources>"#,
                r#"  <Strings>"#,
                r#"    <Hello>Hi!</Hello>"#,
                r#"    <Bye>Byebye!</Bye>"#,
                r#"  </Strings>"#,
                r#"</Resources>"#,
            ],
            "/Resources/Strings/Bye",
            "Byebye!",
            &XmlReplacer,
        );
    }

    #[test]
    fn test_json_replacer_with_jsonpath() {
        test_replacer(
            vec![
                r#"{"#,
                r#"  "Hello": "Hi!", "#,
                r#"  "Bye": "Byebye!""#,
                r#"}"#,
            ],
            "$.Bye",
            "Byebye!",
            &JsonReplacer,
        );
    }

    #[test]
    fn test_json_replacer_with_array() {
        test_replacer(
            vec![
                r#"{"#,
                r#"  "Hello": "Hi!", "#,
                r#"  "Bye": ["Byebye!", "Ja ne!"]"#,
                r#"}"#,
            ],
            "$.Bye[1]",
            "Ja ne!",
            &JsonReplacer,
        );
    }

    #[test]
    fn test_xml_replacer_with_array() {
        test_replacer(
            vec![
                r#"<?xml version="1.0" encoding="UTF-8" ?>"#,
                r#"<Resources>"#,
                r#"  <Strings>"#,
                r#"    <Hello>Hi!</Hello>"#,
                r#"    <Bye>Byebye!</Bye>"#,
                r#"  </Strings>"#,
                r#"  <Strings>"#,
                r#"    <Hello>Ossu!</Hello>"#,
                r#"    <Bye>Ja ne!</Bye>"#,
                r#"  </Strings>"#,
                r#"</Resources>"#,
            ],
            "/Resources/Strings[2]/Bye",
            "Ja ne!",
            &XmlReplacer,
        );
    }

    #[test]
    fn test_json_replacer_with_deeply_nested_path() {
        test_replacer(
            vec![
                r#"{"#,
                r#"  "Hello": "Hi!", "#,
                r#"  "Bye": {"#,
                r#"    "Hello": "Byebye!", "#,
                r#"    "Bye": {"#,
                r#"      "Hello": "Ja ne!""#,
                r#"    }"#,
                r#"  }"#,
                r#"}"#,
            ],
            "$.Bye.Bye.Hello",
            "Ja ne!",
            &JsonReplacer,
        );
    }

    #[test]
    fn test_xml_replacer_with_attribute_selector() {
        test_replacer(
            vec![
                r#"<?xml version="1.0" encoding="UTF-8" ?>"#,
                r#"<Resources>"#,
                r#"  <Strings lang="en">"#,
                r#"    <Hello>Hi!</Hello>"#,
                r#"    <Bye>Byebye!</Bye>"#,
                r#"  </Strings>"#,
                r#"  <Strings lang="jp">"#,
                r#"    <Hello>Ossu!</Hello>"#,
                r#"    <Bye>Ja ne!</Bye>"#,
                r#"  </Strings>"#,
                r#"</Resources>"#,
            ],
            "/Resources/Strings[@lang='jp']/Bye",
            "Ja ne!",
            &XmlReplacer,
        );
    }

    #[test]
    fn test_xml_replacer_with_multiple_matches() {
        test_replacer(
            vec![
                r#"<?xml version="1.0" encoding="UTF-8" ?>"#,
                r#"<Resources>"#,
                r#"  <Strings lang="en">"#,
                r#"    <Hello>Hi!</Hello>"#,
                r#"    <Bye>Byebye!</Bye>"#,
                r#"  </Strings>"#,
                r#"  <Strings lang="jp">"#,
                r#"    <Hello>Ossu!</Hello>"#,
                r#"    <Bye>Ja ne!</Bye>"#,
                r#"  </Strings>"#,
                r#"</Resources>"#,
            ],
            "/Resources/Strings/Bye",
            "Byebye!",
            &XmlReplacer,
        );
    }

    #[test]
    fn test_run() {
        let temp_dir = tempdir().unwrap();
        let resource_file_path = temp_dir.path().join("test.xml");
        let mut resource_file = fs::File::create(&resource_file_path).unwrap();
        let template_extension = ".template";
        let template_file_path = temp_dir
            .path()
            .join(vec!["test", template_extension].join(""));
        let mut template_file = fs::File::create(&template_file_path).unwrap();
        writeln!(resource_file, r#"<?xml version="1.0" encoding="UTF-8" ?>"#).unwrap();
        writeln!(resource_file, r#"<Resources>"#).unwrap();
        writeln!(resource_file, r#"  <Strings lang="en">"#).unwrap();
        writeln!(resource_file, r#"    <Hello>Hi!</Hello>"#).unwrap();
        writeln!(resource_file, r#"    <Bye>Byebye!</Bye>"#).unwrap();
        writeln!(resource_file, r#"  </Strings>"#).unwrap();
        writeln!(resource_file, r#"</Resources>"#).unwrap();
        let template_file_contents = format!(
            "{{{}#/Resources/Strings/Bye}}",
            resource_file_path.into_os_string().into_string().unwrap()
        );
        writeln!(template_file, "{}", template_file_contents).unwrap();
        drop(template_file);
        drop(resource_file);
        let replacers: Vec<Box<dyn ExpressionReplacer>> =
            vec![Box::new(XmlReplacer), Box::new(JsonReplacer)];
        let extension = template_extension.to_string();
        let config = Config {
            extension,
            dir: temp_dir.path().to_path_buf(),
            replacers,
        };
        run(config);
        let template_file_path_without_extension = template_file_path
            .to_str()
            .unwrap()
            .replace(template_extension, "");
        let mut template_file = fs::File::open(template_file_path_without_extension).unwrap();
        let mut template_contents = String::new();
        template_file
            .read_to_string(&mut template_contents)
            .unwrap();
        assert_eq!(
            template_contents, "Byebye!\n",
            "Should replace the first match"
        );
    }
}
