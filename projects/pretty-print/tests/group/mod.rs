use panduck_pp::{text, OpenClosedGroup};

#[test]
fn test_list() {
    let g = OpenClosedGroup::default();
    let a = g.print(
        "[",
        "]",
        vec![
            //
            text("1"),
            text("1"),
            text("1"),
            text("1"),
        ],
    );
    let mut short = String::new();
    let mut long = String::new();
    a.render_fmt(5, &mut short);
    a.render_fmt(20, &mut long);
    println!("{}", short);
    println!("{}", long);
}
