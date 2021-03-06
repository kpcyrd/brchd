use crate::errors::*;
use html5ever::{local_name, parse_document};
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

fn walk(outlinks: &mut Vec<String>, node: &Handle) {
    if let NodeData::Element {
        ref name,
        ref attrs,
        ..
    } = node.data {
        if local_name!("a") == name.local {
            for attr in attrs.borrow().iter() {
                if attr.name.local.eq_str_ignore_ascii_case("href") {
                    outlinks.push(attr.value.to_string());
                }
            }
        }
    }

    for child in node.children.borrow().iter() {
        walk(outlinks, child);
    }
}

pub fn parse_links(bytes: &[u8]) -> Result<Vec<String>> {
    let mut outlinks = Vec::new();
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut &bytes[..])?;
    walk(&mut outlinks, &dom.document);
    Ok(outlinks)
}
