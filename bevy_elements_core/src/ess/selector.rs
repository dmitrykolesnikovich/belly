use bevy::{
    prelude::{default, Changed, Entity, Parent, Query}
};
use smallvec::{SmallVec, smallvec};
use tagstr::Tag;

use crate::Element;


#[derive(Default)]
pub struct SelectorIndex(usize);

impl SelectorIndex {
    pub fn new(value: usize) -> SelectorIndex {
        SelectorIndex(value)
    }
}

pub enum SelectorElement {
    AnyChild,
    Id(Tag),
    Class(Tag),
    Tag(Tag),
    State(Tag),
}

impl SelectorElement {
    pub fn is_any_child(&self) -> bool {
        match self {
            SelectorElement::AnyChild => true,
            _ => false,
        }
    }

    pub fn is_value(&self) -> bool {
        !self.is_any_child()
    }

    pub fn describes_node(&self, node: &impl EmlNode) -> bool {
        match self {
            SelectorElement::Id(id) => node.id() == Some(*id),
            SelectorElement::State(attr) => node.has_state(attr),
            SelectorElement::Tag(tag) => node.tag() == *tag,
            SelectorElement::Class(class) => node.has_class(class),
            _ => false,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            SelectorElement::AnyChild => " ".to_string(),
            SelectorElement::State(s) => format!(":{}", s),
            SelectorElement::Tag(t) => format!("{}", t),
            SelectorElement::Class(c) => format!(".{}", c),
            SelectorElement::Id(i) => format!("#{}", i),
        }
    }
}

pub type SelectorElements = SmallVec<[SelectorElement; 8]>;

pub struct SelectorEntry<'a> {
    offset: usize,
    elements: &'a SelectorElements,
}

impl<'a> SelectorEntry<'a> {
    fn new(elements: &'a SelectorElements) -> SelectorEntry<'a> {
        SelectorEntry {
            elements,
            offset: 0,
        }
    }
    fn next(&self) -> Option<SelectorEntry<'a>> {
        let mut offset = self.offset;
        let elements = self.elements;
        if elements[offset].is_any_child() {
            offset += 1;
            if offset >= elements.len() {
                return None;
            } else {
                return Some(SelectorEntry { offset, elements });
            }
        }

        while offset < elements.len() && !elements[offset].is_any_child() {
            offset += 1;
        }

        if offset >= elements.len() {
            return None;
        } else {
            return Some(SelectorEntry { offset, elements });
        }
    }

    pub fn len(&self) -> u8 {
        let mut len = 0;
        for element in self.elements.iter().skip(self.offset) {
            if element.is_any_child() {
                return len;
            } else {
                len += 1;
            }
        }
        len
    }

    pub fn is_any_child(&self) -> bool {
        self.elements[self.offset].is_any_child()
    }

    pub fn is_value(&self) -> bool {
        !self.is_any_child()
    }

    pub fn has_id(&self, id: Tag) -> bool {
        for element in self.elements.iter().skip(self.offset) {
            match element {
                SelectorElement::AnyChild => return false,
                SelectorElement::Id(element_id) if id == *element_id => return true,
                _ => continue
            }
        }
        false
    }
    
    pub fn has_class(&self, class: Tag) -> bool {
        for element in self.elements.iter().skip(self.offset) {
            match element {
                SelectorElement::AnyChild => return false,
                SelectorElement::Class(element_class) if class == *element_class => return true,
                _ => continue
            }
        }
        false
    }

    pub fn has_tag(&self, tag: Tag) -> bool {
        for element in self.elements.iter().skip(self.offset) {
            match element {
                SelectorElement::AnyChild => return false,
                SelectorElement::Tag(element_tag) if tag == *element_tag => return true,
                _ => continue
            }
        }
        false
    }
    
    pub fn describes_node(&self, node: &impl EmlNode) -> bool {
        let mut offset = self.offset;
        let elements = self.elements;
        if elements[offset].is_any_child() {
            return false;
        }
        while offset < elements.len() && elements[offset].is_value() {
            if elements[offset].describes_node(node) {
                offset += 1
            } else {
                return false;
            }
        }
        true
    }
}

#[derive(Default)]
pub struct Selector {
    pub index: SelectorIndex,
    pub weight: u32,
    pub elements: SelectorElements,
}

impl Selector {
    pub fn new(elements: SelectorElements) -> Selector {
        Selector {
            elements,
            ..default()
        }
    }

    pub fn tail(&self) -> SelectorEntry {
        SelectorEntry {
            offset: 0,
            elements: &self.elements,
        }
    }

    pub fn entries(&self) -> SmallVec<[SelectorEntry; 8]> {
        let mut entries = smallvec![];
        let mut tail = Some(self.tail());
        while let Some(entry) =  tail {
            tail = entry.next();
            if entry.is_value() {
                entries.insert(0, entry);
            }
        }
        entries
    }

    pub fn matches(&self, branch: impl EmlBranch) -> bool {
        let slice = SelectorEntry::new(&self.elements);
        branch.tail().fits(&slice)
    }

    pub fn to_string(&self) -> String {
        let mut result = "".to_string();
        for token in self.elements.iter().rev() {
            result.push_str(&token.to_string());
        }
        result
    }
}

pub trait EmlBranch {
    type Node: EmlNode;
    fn tail(&self) -> Self::Node;
}

pub trait EmlNode: Sized {
    fn id(&self) -> Option<Tag>;
    fn tag(&self) -> Tag;
    fn has_state(&self, tag: &Tag) -> bool;
    fn has_class(&self, class: &Tag) -> bool;

    fn next(&self) -> Option<Self>;

    fn fits(&self, selector: &SelectorEntry) -> bool {
        if selector.is_any_child() {
            let next_selector = selector.next().unwrap();
            if self.fits(&next_selector) {
                return true;
            }
            if let Some(next_node) = self.next() {
                next_node.fits(&next_selector) || next_node.fits(selector)
            } else {
                false
            }
        } else if selector.describes_node(self) {
            match (self.next(), selector.next()) {
                (None, None) => true,
                (Some(next_node), Some(next_slice)) => next_node.fits(&next_slice),
                (Some(_node), None) => true,
                (None, Some(_slice)) => false,
            }
        } else {
            false
        }
    }
}

#[derive(Default)]
pub struct ElementsBranch<'e>(SmallVec<[&'e Element; 12]>);

impl<'e> ElementsBranch<'e> {
    pub fn new() -> ElementsBranch<'e> {
        ElementsBranch::default()
    }

    pub fn insert(&mut self, element: &'e Element) {
        self.0.push(element);
    }

    pub fn to_string(&self) -> String {
        let mut result = "".to_string();
        for (idx, node) in self.0.iter().enumerate().rev() {
            result.push_str(&format!("{}", node.name));
            if let Some(id) = node.id {
                result.push_str(&format!("#{}", id));
            }
            for class in node.classes.iter() {
                result.push_str(&format!(".{}", class));
            }
            for state in node.state.iter() {
                result.push_str(&format!(":{}", state));
            }
            if idx != 0 {
                result.push_str(" ");
            }
        }
        result
    }
}
pub struct ElementNode<'b, 'e> {
    idx: usize,
    branch: &'b ElementsBranch<'e>,
}


impl<'b, 'e> EmlNode for ElementNode<'b, 'e> {
    fn id(&self) -> Option<Tag> {
        self.branch.0[self.idx].id
    }
    fn tag(&self) -> Tag {
        self.branch.0[self.idx].name
    }

    fn has_class(&self, class: &Tag) -> bool {
        self.branch.0[self.idx].classes.contains(class)
    }

    fn has_state(&self, tag: &Tag) -> bool {
        self.branch.0[self.idx].state.contains(tag)
    }

    fn next(&self) -> Option<Self> {
        let idx = self.idx + 1;
        let branch = self.branch;
        if idx >= branch.0.len() {
            None
        } else {
            Some(ElementNode { idx, branch })
        }
    }
}

impl<'b, 'e> EmlBranch for &'b ElementsBranch<'e> {
    type Node = ElementNode<'b, 'e>;

    fn tail(&self) -> Self::Node {
        ElementNode {
            idx: 0,
            branch: *self,
        }
    }
}

fn _example(
    entities: Query<Entity, Changed<Element>>,
    parents: Query<&Parent>,
    elements: Query<&Element>,
) {
    for entity in entities.iter() {
        // build branch for each entity
        let mut branch = smallvec![];
        let mut tail = entity;
        while let Ok(element) = elements.get(tail) {
            branch.push(element);
            if let Ok(parent) = parents.get(tail) {
                tail = parent.get();
            } else {
                break;
            }
        }
        let branch = ElementsBranch(branch);

        // can now find all matching rules
        let selector: Selector = "div span".into();
        if selector.matches(&branch) {
            // apply rules here
        }
    }
}

impl From<&str> for Selector {
    fn from(source: &str) -> Self {
        use cssparser::{Parser, ParserInput, ToCss, Token::*};
        use tagstr::*;
        const NEXT_TAG: u8 = 0;
        const NEXT_CLASS: u8 = 1;
        const NEXT_ATTR: u8 = 2;
        let mut selector = Selector::default();
        // selector.elements.push(SelectorElement::AnyChild);
        let mut input = ParserInput::new(source);
        let mut parser = Parser::new(&mut input);
        let mut next = NEXT_TAG;
        while let Ok(token) = parser.next_including_whitespace() {
            match token {
                Ident(v) => {
                    match next {
                        NEXT_TAG => selector
                            .elements
                            .insert(0, SelectorElement::Tag(v.to_string().as_tag())),
                        NEXT_CLASS => selector
                            .elements
                            .insert(0, SelectorElement::Class(v.to_string().as_tag())),
                        NEXT_ATTR => selector
                            .elements
                            .insert(0, SelectorElement::State(v.to_string().as_tag())),
                        _ => panic!("Invalid NEXT_TAG"),
                    };
                    next = NEXT_TAG;
                }
                IDHash(v) => {
                    if v.is_empty() {
                        panic!("Invalid #id selector");
                    } else {
                        selector
                            .elements
                            .insert(0, SelectorElement::Id(v.to_string().as_tag()));
                    }
                }
                WhiteSpace(_) => selector.elements.insert(0, SelectorElement::AnyChild),
                Colon => next = NEXT_ATTR,
                Delim(c) if *c == '.' => next = NEXT_CLASS,
                _ => panic!("Unexpected token: {}", token.to_css_string()),
            }
        }

        selector
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bevy::utils::HashSet;
    use tagstr::*;
    

    struct TestBranch(Vec<TestNodeData>);

    impl<'a> EmlBranch for &'a TestBranch {
        type Node = TestNode<'a>;

        fn tail(&self) -> Self::Node {
            TestNode {
                index: 0,
                branch: self,
            }
        }
    }

    #[derive(Default)]
    struct TestNodeData {
        id: Option<Tag>,
        tag: Tag,
        classes: HashSet<Tag>,
        attributes: HashSet<Tag>,
    }

    struct TestNode<'a> {
        index: usize,
        branch: &'a TestBranch,
    }

    impl<'a> EmlNode for TestNode<'a> {
        fn id(&self) -> Option<Tag> {
            self.branch.0[self.index].id
        }
        fn tag(&self) -> Tag {
            self.branch.0[self.index].tag
        }
        fn has_state(&self, tag: &Tag) -> bool {
            self.branch.0[self.index].attributes.contains(tag)
        }
        fn has_class(&self, class: &Tag) -> bool {
            self.branch.0[self.index].classes.contains(class)
        }
        fn next(&self) -> Option<Self> {
            let index = self.index + 1;
            if index >= self.branch.0.len() {
                None
            } else {
                Some(TestNode {
                    index,
                    branch: self.branch,
                })
            }
        }
    }

    impl From<Selector> for TestBranch {
        fn from(selector: Selector) -> Self {
            let mut branch = TestBranch(vec![]);
            let mut node = TestNodeData::default();
            let mut has_values = false;
            let void = |_| ();
            for element in selector.elements {
                match element {
                    SelectorElement::AnyChild => {
                        if has_values {
                            branch.0.push(node);
                            node = TestNodeData::default();
                        }
                        has_values = false;
                        continue;
                    }
                    SelectorElement::State(attr) => void(node.attributes.insert(attr)),
                    SelectorElement::Class(class) => void(node.classes.insert(class)),
                    SelectorElement::Id(id) => node.id = Some(id),
                    SelectorElement::Tag(tag) => node.tag = tag,
                };
                has_values = true;
            }
            if has_values {
                branch.0.push(node);
            }
            branch
        }
    }

    impl From<&str> for TestBranch {
        fn from(selector: &str) -> Self {
            let selector: Selector = selector.into();
            selector.into()
        }
    }

    #[test]
    fn selector_construct_test_branch() {
        // single element
        let branch: TestBranch = "div".into();
        assert_eq!(branch.0.len(), 1);

        // spaces
        let branch: TestBranch = "div ".into();
        assert_eq!(branch.0.len(), 1);
        let branch: TestBranch = " div ".into();
        assert_eq!(branch.0.len(), 1);

        // attribute
        let branch: TestBranch = " div:attr ".into();
        assert_eq!(branch.0.len(), 1);
        assert!(branch.0[0].attributes.contains(&"attr".as_tag()));

        // class
        let branch: TestBranch = " div.cls ".into();
        assert_eq!(branch.0.len(), 1);
        assert!(branch.0[0].classes.contains(&"cls".as_tag()));

        // id
        let branch: TestBranch = " div#id ".into();
        assert_eq!(branch.0.len(), 1);
        assert_eq!(branch.0[0].id, Some("id".as_tag()));

        // complex
        let branch: TestBranch = " div#id.cls span:attr ".into();
        assert_eq!(branch.0.len(), 2);
        assert_eq!(branch.0[1].tag, "div".as_tag());
        assert_eq!(branch.0[0].tag, "span".as_tag());
        assert_eq!(branch.0[1].id, Some("id".as_tag()));
        assert_eq!(branch.0[1].classes.contains(&"cls".as_tag()), true);
        assert_eq!(branch.0[0].attributes.contains(&"attr".as_tag()), true);
    }

    #[test]
    fn selector_single_element() {
        let branch: TestBranch = "div".into();
        let valid_selector: Selector = "div".into();
        let invalid_selector: Selector = "span".into();
        assert!(valid_selector.matches(&branch));
        assert!(!invalid_selector.matches(&branch));

        let branch: TestBranch = "div.cls".into();
        let valid_selector: Selector = ".cls".into();
        let invalid_selector: Selector = ":span".into();
        assert!(valid_selector.matches(&branch));
        assert!(!invalid_selector.matches(&branch));
    }

    #[test]
    fn selector_multi_elements() {
        let branch: TestBranch = "div.red#id:pressed span.green span.red".into();
        let valid_selectors: &[&str] = &[
            "span",
            "div span",
            ".red",
            ".green .red",
            "#id:pressed .red",
            "div span span",
            ".red .red",
        ];
        for src in valid_selectors {
            let selector: Selector = src.clone().into();
            assert!(
                selector.matches(&branch),
                "Selector '{}' should be matched",
                src
            );
        }
        let invalid_selectors: &[&str] = &[
            "#id",
            "#id .green",
            "span div",
            "div .green",
            ".red .green",
            ":pressed #id",
            ".red div",
            "#id div",
            "#id.red .red .green",
            "div span span .red",
            ".red .green :pressed",
        ];
        for src in invalid_selectors {
            let selector: Selector = src.clone().into();
            assert!(
                !selector.matches(&branch),
                "Selector '{}' shouldn't be matched",
                src
            );
        }
    }
}
