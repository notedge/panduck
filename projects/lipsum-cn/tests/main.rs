use lipsum_cn::废话生成器;

#[test]
fn ready() {
    println!("it works!")
}

#[test]
fn test() {
    let mut r = 废话生成器::default();
    let out = r.生成文章("废话", 6000);
    println!("{}", out);
}
