use panduck_pp::{text_ref, OpenClosedGroup};

#[test]
fn test_tailing_comma() {
    let mut group = OpenClosedGroup::default();
    let out = group.pretty_render(
        "[",
        "]",
        vec![
            //
            text_ref("1"),
            text_ref("2"),
            text_ref("3"),
            text_ref("4"),
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
            text_ref("1"),
            text_ref("2"),
            text_ref("3"),
            text_ref("4"),
        ],
        5,
    );
    assert_eq!(out, "[\n    1,\n    2,\n    3,\n    4\n]");
}
