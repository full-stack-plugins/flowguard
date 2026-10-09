pub fn answer() -> u32 { core::answer() }
#[cfg(test)]
mod tests {
    #[test]
    fn required_answer() { assert_eq!(super::answer(), 42); }
}
