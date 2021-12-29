use std::collections::HashMap;

fn 加载名言() -> HashMap<usize, &'static str> {
    let mut h = HashMap::new();
    for (i, s) in include_str!("名言.txt").lines().enumerate() {
        let text = s.trim();
        if !text.is_empty() {
            h.insert(i, text);
        }
    }
    return h;
}

#[test]
fn test() {
    let r = 加载名言();
    println!("{:#?}", r);
}
