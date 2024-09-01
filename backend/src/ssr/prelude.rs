use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct PreludeProps {
    children: Element,
}

pub fn Prelude(props: PreludeProps) -> Element {
    rsx! {
        head {
            // TODO: add in CSS stuff and htmx
        }
        body {
            {props.children}
        }
    }
}
