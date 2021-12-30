use panduck_pp::{text, OpenClosedGroup};

#[test]
fn test_tailing_comma() {
    let mut group = OpenClosedGroup::default();
    let out = group.pretty_render(
        "[",
        "]",
        vec![
            //
            text("1"),
            text("2"),
            text("3"),
            text("4"),
        ],
        5,
    );
    assert_eq!(out, "[\n    1,\n    2,\n    3,\n    4,\n]");
    group.set_end_mark(false, false);
    let out = group.pretty_render(
        "[",
        "]",
        vec![
            //
            text("1"),
            text("2"),
            text("3"),
            text("4"),
        ],
        5,
    );
    assert_eq!(out, "[\n    1,\n    2,\n    3,\n    4\n]");
}
