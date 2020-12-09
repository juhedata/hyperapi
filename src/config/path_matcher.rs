use hyper::{Uri, Method};


#[derive(Debug, Clone)]
pub struct RequestMatcher {
    methods: Vec<String>,
    path_pattern: regex::Regex,
}

impl RequestMatcher {
    
    pub fn new(m: String, p: String) -> Self {
        let methods: Vec<String> = m.split(",").map(|s| String::from(s)).collect();
        let path_pattern = regex::Regex::new(&p).unwrap();
        RequestMatcher { methods, path_pattern }
    }

    pub fn is_match(&self, method: &Method, uri: &Uri) -> bool {
        if !self.methods.contains(&method.as_str().into()) {
            return false
        }

        let path = uri.path().strip_prefix("/").unwrap();
        let (_sid, path_left) = path.split_at(path.find("/").unwrap());
        self.path_pattern.is_match(path_left)
    }
}