# xpljn
Simple command-line utility for extracting data from JSON and XML files and inserting it into text files using templates.

Let's say you have a `resources.json` file:
```
{
    "strings": {
        "hello": "Hi!",
        "bye": "Bye!"
    }
}
```
You can create a template file `myfile.js.template` that has some `{file#jsonpath}` expressions inside:
```
let label = "{resources.json#$.strings.hello}";
console.log("I'm a value from JSON file: " + label);
```
When you run this command inside the folder where these 2 files are located:
```
xpljn
```
a new file `myfile.js` will be created with `{file#jsonpath}` expressions replaced with values extracted from the `resources.json` file:
```
let label = "Hi!";
console.log("I'm a value from JSON file: " + label);
```
XML files are also supported, you need to use XPath in expressions:
```
let label = "{resources.xml#/Resources/Strings/Hello}";
console.log("I'm a value from XML file: " + label);
```
You can also mix the two freely in the same template file:
```
let labelXml = "{resources.xml#/Resources/Strings/Hello}";
console.log("I'm a value from XML file: " + labelXml);
let labelJson = "{resources.json#$.strings.hello}";
console.log("I'm a value from JSON file: " + labelJson);
```