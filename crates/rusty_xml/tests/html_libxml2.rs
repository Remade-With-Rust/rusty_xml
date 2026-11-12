//! Issue #12: the HTML parser builds libxml2 2.15's trees.
//!
//! Every expected string below is the output of the pinned oracle
//! (`oracle/bin/xmllint.exe --html`, libxml2 2.15.3) for the same input, with
//! the DOCTYPE line removed.

use rusty_xml::{html_read_memory, xml_save_doc, NodeId, NodeKind, XmlDoc};

fn parse(src: &str) -> XmlDoc {
    html_read_memory(src.as_bytes(), None, None, 0).expect("parses")
}

/// Serialized body of the document, without the DOCTYPE line.
fn html(src: &str) -> String {
    let out = String::from_utf8(xml_save_doc(&parse(src), 0)).expect("utf-8");
    let body = out.split_once('\n').map_or(out.as_str(), |(_, rest)| rest);
    body.trim_end_matches('\n').to_string()
}

#[track_caller]
fn same(src: &str, libxml2: &str) {
    assert_eq!(html(src), libxml2, "input: {src:?}");
}

// ---- 1. implied end tags (htmlStartClose) ----

#[test]
fn start_tags_close_what_the_start_close_table_says() {
    same(
        "<ul><li><p>a<li>b</ul>",
        "<html><body><ul><li><p>a</p></li><li>b</li></ul></body></html>",
    );
    same(
        "<table><tr><td>a<th>b<tr><th>c<td>d</table>",
        "<html><body><table><tr><td>a</td><th>b</th></tr><tr><th>c</th><td>d</td></tr></table></body></html>",
    );
    same(
        "<dl><dt>a<dd>b<dt>c</dl>",
        "<html><body><dl><dt>a</dt><dd>b</dd><dt>c</dt></dl></body></html>",
    );
    same(
        "<select><option>a<option>b</select>",
        "<html><body><select><option>a</option><option>b</option></select></body></html>",
    );
    same(
        "<p><b>x<p>y",
        "<html><body><p><b>x</b></p><p>y</p></body></html>",
    );
}

#[test]
fn html5_sectioning_elements_do_not_close_a_paragraph() {
    // They are not in libxml2's HTML 4 table, so they nest inside the <p>.
    same(
        "<p>a<section>b</section>",
        "<html><body><p>a<section>b</section></p></body></html>",
    );
}

#[test]
fn end_tags_respect_end_priority() {
    // </div> cannot close past <body> (priority 200 > 150): ignored.
    same("<p>a</div>b", "<html><body><p>ab</p></body></html>");
    // </table> (190) closes the open td (160) and tr (170).
    same(
        "<table><tr><td>a</table>z",
        "<html><body><table><tr><td>a</td></tr></table>z</body></html>",
    );
}

#[test]
fn self_closing_syntax_closes_any_element() {
    same(
        "<div/>a<p/>b<br/>c",
        "<html><body><div></div>a<p></p>b<br>c</body></html>",
    );
}

// ---- 2. raw text and RCDATA ----

#[test]
fn script_content_is_one_raw_text_node_in_head() {
    same(
        "<script>var a = \"<b>x</b></div>\"; if (1<2) {}</script><p>after",
        "<html><head><script>var a = \"<b>x</b></div>\"; if (1<2) {}</script></head><body><p>after</p></body></html>",
    );
    let doc = parse("<script>if (1<2) {}</script>");
    let head = doc
        .first_child(doc.xml_doc_get_root_element().expect("html"))
        .expect("head");
    let script = doc.first_child(head).expect("script");
    let raw = doc.first_child(script).expect("content");
    assert_eq!(
        doc.kind(raw),
        NodeKind::CData,
        "libxml2 keeps raw text as HTML_PRESERVE_NODE"
    );
    assert_eq!(doc.content(raw), "if (1<2) {}");
    assert!(doc.next_sibling(raw).is_none());
}

#[test]
fn script_escape_states_nest_script_tags_inside_comments() {
    same(
        "<script>a<!--<script>x</script>y--></script>z",
        "<html><head><script>a<!--<script>x</script>y--></script></head><body>z</body></html>",
    );
}

#[test]
fn title_and_textarea_are_rcdata() {
    same(
        "<title>a <b>x</b></title><p>y",
        "<html><head><title>a &lt;b&gt;x&lt;/b&gt;</title></head><body><p>y</p></body></html>",
    );
    same(
        "<textarea>a&amp;<b></textarea>",
        "<html><body><textarea>a&amp;&lt;b&gt;</textarea></body></html>",
    );
}

#[test]
fn plaintext_runs_to_the_end() {
    same(
        "<plaintext>a</plaintext><b>",
        "<html><body><plaintext>a</plaintext><b></plaintext></body></html>",
    );
}

// ---- 3. character references ----

#[test]
fn unterminated_names_in_attributes_stay_literal() {
    same(
        "<a href=\"?a=1&copy=2&lang=en&amp=3\">x</a>",
        "<html><body><a href=\"?a=1&amp;copy=2&amp;lang=en&amp;amp=3\">x</a></body></html>",
    );
}

#[test]
fn only_legacy_names_may_omit_the_semicolon_in_text() {
    let doc = parse("<p>&lang x &copy y</p>");
    let mut texts = Vec::new();
    collect_text(&doc, NodeId::DOCUMENT, &mut texts);
    assert_eq!(texts, ["&lang x \u{a9} y"]);
}

#[test]
fn numeric_references_follow_libxml2() {
    let doc = parse("<p>&#128;&#0;&#x110000;&#xD800;&#65 &#x41x&#99999999999999;</p>");
    let mut texts = Vec::new();
    collect_text(&doc, NodeId::DOCUMENT, &mut texts);
    assert_eq!(texts, ["\u{20ac}\u{fffd}\u{fffd}\u{fffd}A Ax\u{fffd}"]);
}

fn collect_text(doc: &XmlDoc, id: NodeId, out: &mut Vec<String>) {
    let mut c = doc.first_child(id);
    while let Some(n) = c {
        if doc.kind(n) == NodeKind::Text {
            out.push(doc.content(n).to_string());
        }
        collect_text(doc, n, out);
        c = doc.next_sibling(n);
    }
}

// ---- 4. attributes ----

#[test]
fn first_duplicate_attribute_wins() {
    same(
        "<div a=\"1\" a=\"2\">x</div>",
        "<html><body><div a=\"1\">x</div></body></html>",
    );
}

#[test]
fn valueless_attributes_have_no_value() {
    same(
        "<input value><div hidden data-x>y</div>",
        "<html><body><input value><div hidden data-x>y</div></body></html>",
    );
    let doc = parse("<input value checked>");
    let body = doc
        .first_child(doc.xml_doc_get_root_element().expect("html"))
        .expect("body");
    let input = doc.first_child(body).expect("input");
    assert_eq!(doc.xml_get_prop(input, "value").as_deref(), Some(""));
    // A boolean attribute takes its own name, as xmlSAX2HtmlAttribute does.
    assert_eq!(
        doc.xml_get_prop(input, "checked").as_deref(),
        Some("checked")
    );
}

#[test]
fn attribute_names_follow_htmlparsehtmlname() {
    same(
        "<a b=\"1\"c=2 d=e/f g/=h>t</a>",
        "<html><body><a b=\"1\" c=\"2\" d=\"e/f\" g =h>t</a></body></html>",
    );
}

// ---- 5. line endings ----

#[test]
fn cr_and_crlf_become_lf() {
    same(
        "<p>a\r\nb\rc</p>",
        "<html><body><p>a\nb\nc</p></body></html>",
    );
    same(
        "<textarea>a\r\nb</textarea>",
        "<html><body><textarea>a\nb</textarea></body></html>",
    );
}

// ---- 6. the rest ----

#[test]
fn bogus_markup_becomes_comments() {
    same(
        "<p>x<!foo>y</ 3>z</p>",
        "<html><body><p>x<!--foo-->y<!-- 3-->z</p></body></html>",
    );
}

#[test]
fn frame_is_void() {
    same(
        "<frameset><frame src=a><frame src=b></frameset>",
        "<html><frameset><frame src=\"a\"><frame src=\"b\"></frameset></html>",
    );
}

#[test]
fn head_elements_after_body_content_stay_in_body() {
    same(
        "<p>a</p><title>t</title><meta name=x>",
        "<html><body><p>a</p><title>t</title><meta name=\"x\"></body></html>",
    );
    same(
        "<title>a</title><p>b<title>c</title>",
        "<html><head><title>a</title></head><body><p>b</p><title>c</title></body></html>",
    );
}

#[test]
fn misplaced_structure_tags_are_dropped() {
    same(
        "<div><body class=x><p>q",
        "<html><body><div><p>q</p></div></body></html>",
    );
    same(
        "text<!--c--><head><title>t</title>",
        "<html><body>text<!--c--><title>t</title></body></html>",
    );
}

#[test]
fn content_after_the_root_starts_a_second_root() {
    same(
        "<html><body></body></html><p>x",
        "<html><body></body></html><html><p>x</p></html>",
    );
}

#[test]
fn blanks_around_head_and_body_are_kept() {
    same(
        "<html> <head> <title>t</title> </head> <body> <p>x</p> </body> </html> \n<!--c-->",
        "<html> <head> <title>t</title> </head> <body> <p>x</p> </body> </html><!--c-->",
    );
}
