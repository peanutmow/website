use gloo::utils::document;

pub fn start() {
    crate::water::start();
    let document = document();
    if document.get_element_by_id("archive").is_some() {
        crate::gallery::start();
    }
    if document.get_element_by_id("ascii-water").is_some() {
        crate::landing::start();
    }
    if document.get_element_by_id("bg-water").is_some() {
        crate::blog::start();
    }
    crate::pages::start();
}
