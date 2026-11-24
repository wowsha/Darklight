use super::*;

/// Parses `input` and compares the html5lib-style dump with `expected` (lines without the
/// trailing newline of the last line).
fn check(input: &str, expected: &str) {
    let got = parse(input).dump();
    let want = format!("{}\n", expected.trim_end());
    assert_eq!(got, want, "\ninput: {input:?}");
}

#[test]
fn empty_document_gets_html_head_body() {
    check(
        "",
        "| <html>
|   <head>
|   <body>",
    );
}

#[test]
fn plain_text_is_wrapped_in_body() {
    check(
        "hello",
        "| <html>
|   <head>
|   <body>
|     \"hello\"",
    );
}

#[test]
fn doctype_and_full_document() {
    check(
        "<!DOCTYPE html><html><head><title>T</title></head><body><p>x</p></body></html>",
        "| <!DOCTYPE html>
| <html>
|   <head>
|     <title>
|       \"T\"
|   <body>
|     <p>
|       \"x\"",
    );
}

#[test]
fn head_content_without_head_tag() {
    check(
        "<title>T</title><p>x",
        "| <html>
|   <head>
|     <title>
|       \"T\"
|   <body>
|     <p>
|       \"x\"",
    );
}

#[test]
fn body_content_closes_head_implicitly() {
    check(
        "<meta charset=utf-8><div>x</div>",
        "| <html>
|   <head>
|     <meta>
|       charset=\"utf-8\"
|   <body>
|     <div>
|       \"x\"",
    );
}

#[test]
fn style_in_body_stays_where_it_is_written() {
    check(
        "<body><style>p{}</style><p>x",
        "| <html>
|   <head>
|   <body>
|     <style>
|       \"p{}\"
|     <p>
|       \"x\"",
    );
}

#[test]
fn head_element_after_head_goes_into_head() {
    check(
        "<head></head><meta name=a><body>x",
        "| <html>
|   <head>
|     <meta>
|       name=\"a\"
|   <body>
|     \"x\"",
    );
}

#[test]
fn paragraphs_close_each_other() {
    check(
        "<p>one<p>two<div>three</div>",
        "| <html>
|   <head>
|   <body>
|     <p>
|       \"one\"
|     <p>
|       \"two\"
|     <div>
|       \"three\"",
    );
}

#[test]
fn stray_end_p_creates_empty_paragraph() {
    check(
        "a</p>b",
        "| <html>
|   <head>
|   <body>
|     \"a\"
|     <p>
|     \"b\"",
    );
}

#[test]
fn list_items_close_each_other() {
    check(
        "<ul><li>a<li>b<ul><li>c</ul><li>d</ul>",
        "| <html>
|   <head>
|   <body>
|     <ul>
|       <li>
|         \"a\"
|       <li>
|         \"b\"
|         <ul>
|           <li>
|             \"c\"
|       <li>
|         \"d\"",
    );
}

#[test]
fn definition_list_items_close_each_other() {
    check(
        "<dl><dt>a<dd>b<dt>c</dl>",
        "| <html>
|   <head>
|   <body>
|     <dl>
|       <dt>
|         \"a\"
|       <dd>
|         \"b\"
|       <dt>
|         \"c\"",
    );
}

#[test]
fn headings_do_not_nest() {
    check(
        "<h1>a<h2>b</h2>",
        "| <html>
|   <head>
|   <body>
|     <h1>
|       \"a\"
|     <h2>
|       \"b\"",
    );
}

#[test]
fn end_tag_for_other_heading_level_closes_it() {
    check(
        "<h1>a</h3>b",
        "| <html>
|   <head>
|   <body>
|     <h1>
|       \"a\"
|     \"b\"",
    );
}

#[test]
fn void_elements_and_br_end_tag() {
    check(
        "<p>a<br>b</br>c<img src=x>",
        "| <html>
|   <head>
|   <body>
|     <p>
|       \"a\"
|       <br>
|       \"b\"
|       <br>
|       \"c\"
|       <img>
|         src=\"x\"",
    );
}

#[test]
fn stray_end_tags_are_ignored() {
    check(
        "<div>a</span>b</div>",
        "| <html>
|   <head>
|   <body>
|     <div>
|       \"ab\"",
    );
}

#[test]
fn end_tag_does_not_cross_special_elements() {
    // `</span>` must not close the span outside the div.
    check(
        "<span><div></span>x</div>",
        "| <html>
|   <head>
|   <body>
|     <span>
|       <div>
|         \"x\"",
    );
}

#[test]
fn unclosed_inline_elements_are_closed_by_block_end() {
    check(
        "<div><span>a</div>b",
        "| <html>
|   <head>
|   <body>
|     <div>
|       <span>
|         \"a\"
|     \"b\"",
    );
}

#[test]
fn pre_drops_first_newline() {
    check(
        "<pre>\nline</pre>",
        "| <html>
|   <head>
|   <body>
|     <pre>
|       \"line\"",
    );
}

#[test]
fn textarea_is_rcdata_and_drops_first_newline() {
    check(
        "<textarea>\n<b>&amp;</textarea>x",
        "| <html>
|   <head>
|   <body>
|     <textarea>
|       \"<b>&\"
|     \"x\"",
    );
}

#[test]
fn comments_in_every_position() {
    check(
        "<!--a--><html><!--b--><head><!--c--></head><body><!--d--></body></html><!--e-->",
        "| <!-- a -->
| <html>
|   <!-- b -->
|   <head>
|     <!-- c -->
|   <body>
|     <!-- d -->
| <!-- e -->",
    );
}

#[test]
fn whitespace_placement_in_a_pretty_printed_document() {
    // Whitespace after `<head>` stays in the head, whitespace between `</head>` and `<body>`
    // goes in `<html>`, and whitespace after `</body>` / `</html>` is handled by the in-body
    // rules, so it ends up in `<body>`.
    check(
        "<html>\n<head>\n</head>\n<body>\nx\n</body>\n</html>\n",
        "| <html>
|   <head>
|     \"\n\"
|   \"\n\"
|   <body>
|     \"\nx\n\n\n\"",
    );
}

#[test]
fn duplicate_html_and_body_attributes_merge() {
    check(
        "<html a=1><body b=1><html a=2 c=3><body b=2 d=4>",
        "| <html>
|   a=\"1\"
|   c=\"3\"
|   <head>
|   <body>
|     b=\"1\"
|     d=\"4\"",
    );
}

#[test]
fn content_after_body_end_goes_back_into_body() {
    // Adjacent text merges into one node, as in the DOM.
    check(
        "<body>a</body>b<p>c</html>d",
        "| <html>
|   <head>
|   <body>
|     \"ab\"
|     <p>
|       \"cd\"",
    );
}

#[test]
fn tables_close_cells_and_rows_implicitly() {
    check(
        "<table><tr><td>a<td>b<tr><td>c</table>d",
        "| <html>
|   <head>
|   <body>
|     <table>
|       <tr>
|         <td>
|           \"a\"
|         <td>
|           \"b\"
|       <tr>
|         <td>
|           \"c\"
|     \"d\"",
    );
}

#[test]
fn nested_table_cell_does_not_close_outer_cell() {
    check(
        "<table><tr><td><table><tr><td>in</table>out</table>",
        "| <html>
|   <head>
|   <body>
|     <table>
|       <tr>
|         <td>
|           <table>
|             <tr>
|               <td>
|                 \"in\"
|           \"out\"",
    );
}

#[test]
fn button_start_closes_open_button() {
    check(
        "<button>a<button>b",
        "| <html>
|   <head>
|   <body>
|     <button>
|       \"a\"
|     <button>
|       \"b\"",
    );
}

#[test]
fn image_is_img() {
    check(
        "<image src=a>",
        "| <html>
|   <head>
|   <body>
|     <img>
|       src=\"a\"",
    );
}

#[test]
fn script_content_is_not_markup() {
    check(
        "<script>if (a<b) {x = \"</p>\"}</script><p>y",
        "| <html>
|   <head>
|     <script>
|       \"if (a<b) {x = \"</p>\"}\"
|   <body>
|     <p>
|       \"y\"",
    );
}

#[test]
fn entities_in_text_and_attributes() {
    check(
        "<p title=\"a&amp;b\">&lt;&#65;&nbsp;</p>",
        "| <html>
|   <head>
|   <body>
|     <p>
|       title=\"a&b\"
|       \"<A\u{a0}\"",
    );
}

#[test]
fn nesting_depth_is_bounded() {
    let html = "<div>".repeat(MAX_DEPTH * 4);
    let doc = parse(&html);
    // No element may have more than MAX_DEPTH ancestors.
    let mut deepest = 0;
    for id in doc.descendants(doc.root()) {
        let mut depth = 0;
        let mut cur = id;
        while let Some(parent) = doc.node(cur).parent {
            depth += 1;
            cur = parent;
        }
        deepest = deepest.max(depth);
    }
    assert!(deepest <= MAX_DEPTH + 1, "depth {deepest}");
    // All the divs are still present, just flattened.
    assert_eq!(
        doc.descendants(doc.root())
            .into_iter()
            .filter(|&id| matches!(&doc.node(id).data, NodeData::Element(e) if e.tag == "div"))
            .count(),
        MAX_DEPTH * 4
    );
}

#[test]
fn deep_end_tags_after_flattening_do_not_panic() {
    let html = format!("{}x{}", "<div>".repeat(2000), "</div>".repeat(2000));
    let doc = parse(&html);
    assert!(doc.text_content(doc.root()).contains('x'));
}

/// Every tree must satisfy the arena invariants no matter what the input was.
fn assert_well_formed(doc: &Document) {
    assert!(matches!(doc.node(doc.root()).data, NodeData::Document));
    for id in doc.descendants(doc.root()) {
        let node = doc.node(id);
        for &child in &node.children {
            assert_eq!(doc.node(child).parent, Some(id), "parent link of {child}");
        }
        if id != doc.root() {
            assert!(node.parent.is_some());
        }
    }
    // The root of every parsed document has exactly one <html> element.
    let htmls = doc
        .node(doc.root())
        .children
        .iter()
        .filter(|&&c| matches!(&doc.node(c).data, NodeData::Element(e) if e.tag == "html"))
        .count();
    assert_eq!(htmls, 1);
    // ...which has a head followed by a body.
    let html = doc.find_element("html").unwrap();
    let kids: Vec<&str> = doc
        .node(html)
        .children
        .iter()
        .filter_map(|&c| match &doc.node(c).data {
            NodeData::Element(e) => Some(e.tag.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(kids.first(), Some(&"head"), "{kids:?}");
    assert_eq!(kids.get(1), Some(&"body"), "{kids:?}");
}

#[test]
fn never_panics_and_stays_well_formed_on_garbage() {
    // A small deterministic generator (xorshift) so failures reproduce.
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    const PIECES: &[&str] = &[
        "<",
        ">",
        "</",
        "/>",
        "<!--",
        "-->",
        "<!",
        "<!DOCTYPE",
        "&",
        "&amp;",
        "&#",
        ";",
        "=",
        "\"",
        "'",
        " ",
        "\n",
        "a",
        "é",
        "✨",
        "<p>",
        "</p>",
        "<div>",
        "</div>",
        "<li>",
        "<ul>",
        "</ul>",
        "<table>",
        "<tr>",
        "<td>",
        "</table>",
        "<h1>",
        "</h3>",
        "<b>",
        "</b>",
        "<br>",
        "<img src=x>",
        "<script>",
        "</script>",
        "<style>",
        "</style>",
        "<title>",
        "</title>",
        "<textarea>",
        "</textarea>",
        "<pre>",
        "<html>",
        "</html>",
        "<head>",
        "</head>",
        "<body>",
        "</body>",
        "<button>",
        "<form>",
        "<dd>",
        "<dt>",
        "<option>",
        "<a href=x>",
        "</a>",
        "<span class=a>",
    ];
    for _ in 0..3000 {
        let n = (next() % 40) as usize;
        let mut input = String::new();
        for _ in 0..n {
            input.push_str(PIECES[(next() % PIECES.len() as u64) as usize]);
        }
        let doc = parse(&input);
        assert_well_formed(&doc);
    }
}
