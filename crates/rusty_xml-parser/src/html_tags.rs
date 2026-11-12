//! Element facts for the HTML tree builder, as small integer tags.
//!
//! libxml2 answers "does `<new>` close the open `<old>`?" with a `bsearch` and
//! two `strcmp`s over `htmlStartClose`, and "is it empty?" with another
//! `bsearch` over `html40ElementTable`. Every start tag asks the first question
//! once per element it pops and the second once more. Here a name is interned
//! to a [`Tag`] once, when it is read, and every later question is one load and
//! one bit test against a table built at compile time from the same data.
//!
//! The tables are transcribed from libxml2 2.15.3 `HTMLparser.c`
//! (`html40ElementTable`, `htmlStartClose`, `htmlEndPriority`).

/// Index into [`NAMES`]; `0` is any element libxml2 has no entry for.
pub(crate) type Tag = u8;

/// Unknown element: no table row, closes nothing, is closed by nothing.
pub(crate) const UNKNOWN: Tag = 0;

/// Every element name libxml2 knows, sorted bytewise. `""` holds slot 0 so a
/// tag is its index. `listing` has no element row but appears in the
/// start-close table.
pub(crate) static NAMES: [&str; NT] = [
    "",
    "a",
    "abbr",
    "acronym",
    "address",
    "applet",
    "area",
    "b",
    "base",
    "basefont",
    "bdo",
    "bgsound",
    "big",
    "blockquote",
    "body",
    "br",
    "button",
    "caption",
    "center",
    "cite",
    "code",
    "col",
    "colgroup",
    "dd",
    "del",
    "dfn",
    "dir",
    "div",
    "dl",
    "dt",
    "em",
    "embed",
    "fieldset",
    "font",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "hr",
    "html",
    "i",
    "iframe",
    "img",
    "input",
    "ins",
    "isindex",
    "kbd",
    "keygen",
    "label",
    "legend",
    "li",
    "link",
    "listing",
    "map",
    "menu",
    "meta",
    "noembed",
    "noframes",
    "noscript",
    "object",
    "ol",
    "optgroup",
    "option",
    "p",
    "param",
    "plaintext",
    "pre",
    "q",
    "s",
    "samp",
    "script",
    "select",
    "small",
    "source",
    "span",
    "strike",
    "strong",
    "style",
    "sub",
    "sup",
    "table",
    "tbody",
    "td",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "tt",
    "u",
    "ul",
    "var",
    "wbr",
    "xmp",
];

/// Number of tags, including [`UNKNOWN`]. At most 128, so a start-close row is
/// one `u128`.
pub(crate) const NT: usize = 102;

const fn name_at(i: usize) -> &'static str {
    NAMES[i]
}

/// The element name for a known tag.
pub(crate) fn tag_name(t: Tag) -> &'static str {
    name_at(t as usize)
}

const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Compile-time name -> tag. Panics at compile time on a typo.
const fn t(name: &str) -> Tag {
    let mut i = 1;
    while i < NT {
        if str_eq(name_at(i), name) {
            return i as Tag;
        }
        i += 1;
    }
    panic!("unknown tag in html_tags table")
}

pub(crate) const HTML: Tag = t("html");
pub(crate) const HEAD: Tag = t("head");
pub(crate) const BODY: Tag = t("body");
pub(crate) const P: Tag = t("p");

/// Content model of an element's text, `htmlElemDesc.dataMode`.
pub(crate) const DATA_RCDATA: u8 = 1;
pub(crate) const DATA_RAWTEXT: u8 = 2;
pub(crate) const DATA_PLAINTEXT: u8 = 3;
pub(crate) const DATA_SCRIPT: u8 = 4;

const F_EMPTY: u8 = 0x08;
const F_MODE: u8 = 0x07;
/// head-type elements that `htmlCheckImplied` routes into `<head>`.
const F_HEADISH: u8 = 0x10;
/// noframes / frame / frameset: no implied `<body>`.
const F_FRAMEISH: u8 = 0x20;

const FLAGS: [u8; NT] = {
    let mut f = [0u8; NT];
    let empty = [
        "area", "base", "basefont", "bgsound", "br", "col", "embed", "frame", "hr", "img", "input",
        "isindex", "keygen", "link", "meta", "param", "source", "track", "wbr",
    ];
    let mut i = 0;
    while i < empty.len() {
        f[t(empty[i]) as usize] |= F_EMPTY;
        i += 1;
    }
    f[t("iframe") as usize] |= DATA_RAWTEXT;
    f[t("noembed") as usize] |= DATA_RAWTEXT;
    f[t("noframes") as usize] |= DATA_RAWTEXT;
    f[t("style") as usize] |= DATA_RAWTEXT;
    f[t("xmp") as usize] |= DATA_RAWTEXT;
    f[t("plaintext") as usize] |= DATA_PLAINTEXT;
    f[t("script") as usize] |= DATA_SCRIPT;
    f[t("textarea") as usize] |= DATA_RCDATA;
    f[t("title") as usize] |= DATA_RCDATA;
    let headish = ["script", "style", "meta", "link", "title", "base"];
    i = 0;
    while i < headish.len() {
        f[t(headish[i]) as usize] |= F_HEADISH;
        i += 1;
    }
    f[t("noframes") as usize] |= F_FRAMEISH;
    f[t("frame") as usize] |= F_FRAMEISH;
    f[t("frameset") as usize] |= F_FRAMEISH;
    f
};

#[inline]
pub(crate) fn is_empty(t: Tag) -> bool {
    FLAGS[t as usize] & F_EMPTY != 0
}
#[inline]
pub(crate) fn data_mode(t: Tag) -> u8 {
    FLAGS[t as usize] & F_MODE
}
#[inline]
pub(crate) fn is_headish(t: Tag) -> bool {
    FLAGS[t as usize] & F_HEADISH != 0
}
#[inline]
pub(crate) fn is_frameish(t: Tag) -> bool {
    FLAGS[t as usize] & F_FRAMEISH != 0
}

/// `htmlEndPriority`: an end tag only closes elements of lower or equal
/// priority on its way down the stack.
const PRIORITY: [u8; NT] = {
    let mut p = [100u8; NT];
    p[t("div") as usize] = 150;
    p[t("td") as usize] = 160;
    p[t("th") as usize] = 160;
    p[t("tr") as usize] = 170;
    p[t("thead") as usize] = 180;
    p[t("tbody") as usize] = 180;
    p[t("tfoot") as usize] = 180;
    p[t("table") as usize] = 190;
    p[t("head") as usize] = 200;
    p[t("body") as usize] = 200;
    p[t("html") as usize] = 220;
    p
};

#[inline]
pub(crate) fn end_priority(t: Tag) -> u8 {
    PRIORITY[t as usize]
}

/// `htmlStartClose`, (old, new): a start tag `new` implies the end of an open
/// `old`. Transcribed in libxml2's order.
const START_CLOSE_PAIRS: &[(&str, &str)] = &[
    ("a", "a"),
    ("a", "fieldset"),
    ("a", "table"),
    ("a", "td"),
    ("a", "th"),
    ("address", "dd"),
    ("address", "dl"),
    ("address", "dt"),
    ("address", "form"),
    ("address", "li"),
    ("address", "ul"),
    ("b", "center"),
    ("b", "p"),
    ("b", "td"),
    ("b", "th"),
    ("big", "p"),
    ("caption", "col"),
    ("caption", "colgroup"),
    ("caption", "tbody"),
    ("caption", "tfoot"),
    ("caption", "thead"),
    ("caption", "tr"),
    ("col", "col"),
    ("col", "colgroup"),
    ("col", "tbody"),
    ("col", "tfoot"),
    ("col", "thead"),
    ("col", "tr"),
    ("colgroup", "colgroup"),
    ("colgroup", "tbody"),
    ("colgroup", "tfoot"),
    ("colgroup", "thead"),
    ("colgroup", "tr"),
    ("dd", "dt"),
    ("dir", "dd"),
    ("dir", "dl"),
    ("dir", "dt"),
    ("dir", "form"),
    ("dir", "ul"),
    ("dl", "form"),
    ("dl", "li"),
    ("dt", "dd"),
    ("dt", "dl"),
    ("font", "center"),
    ("font", "td"),
    ("font", "th"),
    ("form", "form"),
    ("h1", "fieldset"),
    ("h1", "form"),
    ("h1", "li"),
    ("h1", "p"),
    ("h1", "table"),
    ("h2", "fieldset"),
    ("h2", "form"),
    ("h2", "li"),
    ("h2", "p"),
    ("h2", "table"),
    ("h3", "fieldset"),
    ("h3", "form"),
    ("h3", "li"),
    ("h3", "p"),
    ("h3", "table"),
    ("h4", "fieldset"),
    ("h4", "form"),
    ("h4", "li"),
    ("h4", "p"),
    ("h4", "table"),
    ("h5", "fieldset"),
    ("h5", "form"),
    ("h5", "li"),
    ("h5", "p"),
    ("h5", "table"),
    ("h6", "fieldset"),
    ("h6", "form"),
    ("h6", "li"),
    ("h6", "p"),
    ("h6", "table"),
    ("head", "a"),
    ("head", "abbr"),
    ("head", "acronym"),
    ("head", "address"),
    ("head", "b"),
    ("head", "bdo"),
    ("head", "big"),
    ("head", "blockquote"),
    ("head", "body"),
    ("head", "br"),
    ("head", "center"),
    ("head", "cite"),
    ("head", "code"),
    ("head", "dd"),
    ("head", "dfn"),
    ("head", "dir"),
    ("head", "div"),
    ("head", "dl"),
    ("head", "dt"),
    ("head", "em"),
    ("head", "fieldset"),
    ("head", "font"),
    ("head", "form"),
    ("head", "frameset"),
    ("head", "h1"),
    ("head", "h2"),
    ("head", "h3"),
    ("head", "h4"),
    ("head", "h5"),
    ("head", "h6"),
    ("head", "hr"),
    ("head", "i"),
    ("head", "iframe"),
    ("head", "img"),
    ("head", "kbd"),
    ("head", "li"),
    ("head", "listing"),
    ("head", "map"),
    ("head", "menu"),
    ("head", "ol"),
    ("head", "p"),
    ("head", "pre"),
    ("head", "q"),
    ("head", "s"),
    ("head", "samp"),
    ("head", "small"),
    ("head", "span"),
    ("head", "strike"),
    ("head", "strong"),
    ("head", "sub"),
    ("head", "sup"),
    ("head", "table"),
    ("head", "tt"),
    ("head", "u"),
    ("head", "ul"),
    ("head", "var"),
    ("head", "xmp"),
    ("hr", "form"),
    ("i", "center"),
    ("i", "p"),
    ("i", "td"),
    ("i", "th"),
    ("legend", "fieldset"),
    ("li", "li"),
    ("link", "body"),
    ("link", "frameset"),
    ("listing", "dd"),
    ("listing", "dl"),
    ("listing", "dt"),
    ("listing", "fieldset"),
    ("listing", "form"),
    ("listing", "li"),
    ("listing", "table"),
    ("listing", "ul"),
    ("menu", "dd"),
    ("menu", "dl"),
    ("menu", "dt"),
    ("menu", "form"),
    ("menu", "ul"),
    ("ol", "form"),
    ("option", "optgroup"),
    ("option", "option"),
    ("p", "address"),
    ("p", "blockquote"),
    ("p", "body"),
    ("p", "caption"),
    ("p", "center"),
    ("p", "col"),
    ("p", "colgroup"),
    ("p", "dd"),
    ("p", "dir"),
    ("p", "div"),
    ("p", "dl"),
    ("p", "dt"),
    ("p", "fieldset"),
    ("p", "form"),
    ("p", "frameset"),
    ("p", "h1"),
    ("p", "h2"),
    ("p", "h3"),
    ("p", "h4"),
    ("p", "h5"),
    ("p", "h6"),
    ("p", "head"),
    ("p", "hr"),
    ("p", "li"),
    ("p", "listing"),
    ("p", "menu"),
    ("p", "ol"),
    ("p", "p"),
    ("p", "pre"),
    ("p", "table"),
    ("p", "tbody"),
    ("p", "td"),
    ("p", "tfoot"),
    ("p", "th"),
    ("p", "title"),
    ("p", "tr"),
    ("p", "ul"),
    ("p", "xmp"),
    ("pre", "dd"),
    ("pre", "dl"),
    ("pre", "dt"),
    ("pre", "fieldset"),
    ("pre", "form"),
    ("pre", "li"),
    ("pre", "table"),
    ("pre", "ul"),
    ("s", "p"),
    ("script", "noscript"),
    ("small", "p"),
    ("span", "td"),
    ("span", "th"),
    ("strike", "p"),
    ("style", "body"),
    ("style", "frameset"),
    ("tbody", "tbody"),
    ("tbody", "tfoot"),
    ("td", "tbody"),
    ("td", "td"),
    ("td", "tfoot"),
    ("td", "th"),
    ("td", "tr"),
    ("tfoot", "tbody"),
    ("th", "tbody"),
    ("th", "td"),
    ("th", "tfoot"),
    ("th", "th"),
    ("th", "tr"),
    ("thead", "tbody"),
    ("thead", "tfoot"),
    ("title", "body"),
    ("title", "frameset"),
    ("tr", "tbody"),
    ("tr", "tfoot"),
    ("tr", "tr"),
    ("tt", "p"),
    ("u", "p"),
    ("u", "td"),
    ("u", "th"),
    ("ul", "address"),
    ("ul", "form"),
    ("ul", "menu"),
    ("ul", "pre"),
    ("xmp", "dd"),
    ("xmp", "dl"),
    ("xmp", "dt"),
    ("xmp", "fieldset"),
    ("xmp", "form"),
    ("xmp", "li"),
    ("xmp", "table"),
    ("xmp", "ul"),
];

/// Row `old`, bit `new`: does start tag `new` close an open `old`?
static START_CLOSE: [u128; NT] = {
    assert!(NT <= 128, "a start-close row must fit one u128");
    let mut rows = [0u128; NT];
    let mut i = 0;
    while i < START_CLOSE_PAIRS.len() {
        let (old, new) = START_CLOSE_PAIRS[i];
        rows[t(old) as usize] |= 1u128 << t(new);
        i += 1;
    }
    rows
};

/// `htmlCheckAutoClose(new, old)`.
#[inline]
pub(crate) fn start_closes(new: Tag, old: Tag) -> bool {
    (START_CLOSE[old as usize] >> new) & 1 != 0
}

/// Longest known name (`blockquote`).
const MAX_TAG_LEN: usize = 10;
const HASH_SLOTS: usize = 512;

/// A hash that is collision-free over [`NAMES`] (constants found by search;
/// the table build below fails to compile if that ever stops being true).
#[inline]
const fn tag_hash(b: &[u8]) -> usize {
    let n = b.len();
    let b1 = if n > 1 { b[1] } else { b[0] };
    (b[0] as usize * 7 + b1 as usize * 6 + b[n - 1] as usize * 27 + n * 34) % HASH_SLOTS
}

static TAG_HASH: [Tag; HASH_SLOTS] = {
    let mut slots = [UNKNOWN; HASH_SLOTS];
    let mut i = 1;
    while i < NT {
        let h = tag_hash(NAMES[i].as_bytes());
        assert!(slots[h] == UNKNOWN, "tag hash collision");
        assert!(NAMES[i].len() <= MAX_TAG_LEN);
        slots[h] = i as Tag;
        i += 1;
    }
    slots
};

/// Intern a lowercased element name: one hash, one load, one compare. A
/// binary search over [`NAMES`] (up to seven memcmp calls) cost +10.7% /
/// +13.1% parse Ir (fragments / pages) -- every start and end tag pays it.
#[inline]
pub(crate) fn tag_of(name: &[u8]) -> Tag {
    if name.is_empty() || name.len() > MAX_TAG_LEN {
        return UNKNOWN;
    }
    let t = TAG_HASH[tag_hash(name)];
    if NAMES[t as usize].as_bytes() == name {
        t
    } else {
        UNKNOWN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_sorted_and_every_name_interns_to_itself() {
        for i in 1..NT {
            assert_eq!(tag_of(tag_name(i as Tag).as_bytes()) as usize, i);
            if i > 1 {
                assert!(
                    tag_name((i - 1) as Tag) < tag_name(i as Tag),
                    "{}",
                    tag_name(i as Tag)
                );
            }
        }
        assert_eq!(tag_of(b"custom-element"), UNKNOWN);
        assert_eq!(tag_of(b""), UNKNOWN);
    }

    #[test]
    fn start_close_matches_the_pairs() {
        assert!(start_closes(t("li"), t("li")));
        assert!(start_closes(t("dd"), t("dt")));
        assert!(start_closes(t("body"), HEAD));
        assert!(
            !start_closes(tag_of(b"section"), P),
            "section is not an HTML 4 element"
        );
        assert!(!start_closes(t("li"), t("ul")));
        assert!(!start_closes(UNKNOWN, P));
        assert!(!start_closes(P, UNKNOWN));
    }
}
