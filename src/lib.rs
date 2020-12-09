pub mod proxy;
pub mod config;
pub mod middleware;


#[cfg(test)]
mod tests {
    #[test]
    fn exploration() {
        assert_eq!(2 + 2, 4);
    }
}

