use gitctx_core::parser::AstExtractor;
use gitctx_core::qualname::QualName;
use std::path::Path;

#[test]
fn test_python_qualnames() {
    let extractor = AstExtractor::new();
    let code = r"
def outer_func():
    def inner_func():
        f = lambda x: x + 1
        return f(10)
    return inner_func()

class UserAccount:
    def get_balance(self):
        return 100
";
    let res = extractor
        .parse_blob(Path::new("test.py"), code.as_bytes(), None)
        .expect("parse python");

    let names: Vec<String> = res.symbols.iter().map(|s| s.qualname.clone()).collect();
    assert!(names.contains(&"outer_func".to_string()));
    assert!(names.contains(&"outer_func::<inner_func>".to_string()));
    assert!(names
        .iter()
        .any(|n| n.starts_with("outer_func::<inner_func>::<closure@L")));
    assert!(names.contains(&"UserAccount::get_balance".to_string()));

    let method = res
        .symbols
        .iter()
        .find(|s| s.name == "get_balance")
        .unwrap();
    assert!(method.is_method);
    let qual = QualName::new(&method.qualname);
    assert_eq!(qual.parent(), Some("UserAccount".to_string()));
    assert_eq!(qual.leaf(), "get_balance");
    assert!(qual.is_method());
    assert!(!qual.is_nested_or_closure());

    let inner = res.symbols.iter().find(|s| s.name == "inner_func").unwrap();
    assert!(!inner.is_method);
    let qual_inner = QualName::new(&inner.qualname);
    assert_eq!(qual_inner.parent(), Some("outer_func".to_string()));
    assert_eq!(qual_inner.leaf(), "<inner_func>");
    assert!(qual_inner.is_nested_or_closure());
}

#[test]
fn test_rust_qualnames() {
    let extractor = AstExtractor::new();
    let code = r"
fn top_level() {
    fn local_helper() {
        let _closure = |x: i32| x * 2;
    }
}

struct Service;

impl Service {
    pub fn execute(&self) {}
}
";
    let res = extractor
        .parse_blob(Path::new("test.rs"), code.as_bytes(), None)
        .expect("parse rust");

    let names: Vec<String> = res.symbols.iter().map(|s| s.qualname.clone()).collect();
    assert!(names.contains(&"top_level".to_string()));
    assert!(names.contains(&"top_level::<local_helper>".to_string()));
    assert!(names
        .iter()
        .any(|n| n.starts_with("top_level::<local_helper>::<closure@L")));
    assert!(names.contains(&"Service::execute".to_string()));

    let exec = res.symbols.iter().find(|s| s.name == "execute").unwrap();
    assert!(exec.is_method);
    let qual = QualName::new(&exec.qualname);
    assert!(qual.is_method());
    assert_eq!(qual.leaf(), "execute");
    assert_eq!(qual.parent(), Some("Service".to_string()));
}

#[test]
fn test_javascript_and_typescript_qualnames() {
    let extractor = AstExtractor::new();

    // JavaScript
    let js_code = r"
class Controller {
    handleRequest() {
        const callback = (data) => data.id;
    }
}
function standalone() {
    function nested() {}
}
";
    let js_res = extractor
        .parse_blob(Path::new("app.js"), js_code.as_bytes(), None)
        .expect("parse js");
    let js_names: Vec<String> = js_res.symbols.iter().map(|s| s.qualname.clone()).collect();
    assert!(js_names.contains(&"Controller::handleRequest".to_string()));
    assert!(js_names
        .iter()
        .any(|n| n.starts_with("Controller::handleRequest::<closure@L")));
    assert!(js_names.contains(&"standalone".to_string()));
    assert!(js_names.contains(&"standalone::<nested>".to_string()));

    // TypeScript
    let ts_code = r"
class DatabaseService {
    query<T>(sql: string): T[] {
        const mapper = (row: any): T => row;
        return [];
    }
}
";
    let ts_res = extractor
        .parse_blob(Path::new("db.ts"), ts_code.as_bytes(), None)
        .expect("parse ts");
    let ts_names: Vec<String> = ts_res.symbols.iter().map(|s| s.qualname.clone()).collect();
    assert!(ts_names.contains(&"DatabaseService::query".to_string()));
    assert!(ts_names
        .iter()
        .any(|n| n.starts_with("DatabaseService::query::<closure@L")));
}

#[test]
fn test_go_qualnames() {
    let extractor = AstExtractor::new();
    let code = r#"
package main

type Server struct{}

func (s *Server) Start(port int) error {
    handler := func() {
        println("handling")
    }
    handler()
    return nil
}

func StandaloneFunc() {}
"#;
    let res = extractor
        .parse_blob(Path::new("main.go"), code.as_bytes(), None)
        .expect("parse go");

    let names: Vec<String> = res.symbols.iter().map(|s| s.qualname.clone()).collect();
    assert!(names.contains(&"Server::Start".to_string()));
    assert!(names
        .iter()
        .any(|n| n.starts_with("Server::Start::<closure@L")));
    assert!(names.contains(&"StandaloneFunc".to_string()));

    let start = res.symbols.iter().find(|s| s.name == "Start").unwrap();
    assert!(start.is_method);
    let qual = QualName::new(&start.qualname);
    assert_eq!(qual.parent(), Some("Server".to_string()));
    assert_eq!(qual.leaf(), "Start");
}

#[test]
fn test_c_and_cpp_qualnames() {
    let extractor = AstExtractor::new();

    // C
    let c_code = "int process_data(int x) { return x * 2; }";
    let c_res = extractor
        .parse_blob(Path::new("main.c"), c_code.as_bytes(), None)
        .expect("parse c");
    assert_eq!(c_res.symbols[0].qualname, "process_data");

    // C++
    let cpp_code = r"
namespace graphics {
    class Renderer {
    public:
        void draw() {
            auto lambda = []() { return 1; };
        }
    };
}
";
    let cpp_res = extractor
        .parse_blob(Path::new("render.cpp"), cpp_code.as_bytes(), None)
        .expect("parse cpp");
    let cpp_names: Vec<String> = cpp_res.symbols.iter().map(|s| s.qualname.clone()).collect();
    assert!(cpp_names.contains(&"graphics::Renderer::draw".to_string()));
    assert!(cpp_names
        .iter()
        .any(|n| n.starts_with("graphics::Renderer::draw::<closure@L")));
}

#[test]
fn test_java_and_php_qualnames() {
    let extractor = AstExtractor::new();

    // Java
    let java_code = r"
public class OrderManager {
    public void processOrder(long id) {
        Runnable r = () -> { System.out.println(id); };
    }
}
";
    let java_res = extractor
        .parse_blob(Path::new("OrderManager.java"), java_code.as_bytes(), None)
        .expect("parse java");
    let java_names: Vec<String> = java_res
        .symbols
        .iter()
        .map(|s| s.qualname.clone())
        .collect();
    assert!(java_names.contains(&"OrderManager::processOrder".to_string()));
    assert!(java_names
        .iter()
        .any(|n| n.starts_with("OrderManager::processOrder::<closure@L")));

    // PHP
    let php_code = r"
<?php
class UserRepository {
    public function findById($id) {
        $filter = function($item) { return $item->id == 1; };
        return null;
    }
}
";
    let php_res = extractor
        .parse_blob(Path::new("UserRepo.php"), php_code.as_bytes(), None)
        .expect("parse php");
    let php_names: Vec<String> = php_res.symbols.iter().map(|s| s.qualname.clone()).collect();
    assert!(php_names.contains(&"UserRepository::findById".to_string()));
    assert!(php_names
        .iter()
        .any(|n| n.starts_with("UserRepository::findById::<closure@L")));
}
