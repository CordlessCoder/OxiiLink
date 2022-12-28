use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

pub fn highlight_to_html(
    data: &str,
    ss: &SyntaxSet,
    syntax: &SyntaxReference,
) -> String {
    let mut html = String::with_capacity(data.len() + data.len() / 2);
    html.push_str("<!DOCTYPE html>\n<html><head>\n<link rel=\"stylesheet\" href=\"/files/maintheme.css\"></head><body>");
    let mut html_generator =
        ClassedHTMLGenerator::new_with_class_style(syntax, &ss, ClassStyle::Spaced);
    for line in LinesWithEndings::from(data) {
        html_generator
            .parse_html_for_line_which_includes_newline(line)
            .unwrap();
    }
    let html_n = html_generator.finalize();

    html.push_str("<pre class=\"code\">");
    html.push_str(&html_n);
    html.push_str("</pre>\n</body></html>");
    // let Ok(html) = highlighted_html_for_string(data, ss, syntax, theme) else {
    //     return None
    // };
    html
}
