use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

pub fn highlight_to_html(
    data: &str,
    ss: &SyntaxSet,
    syntax: &SyntaxReference,
    extra: &str,
) -> String {
    let mut html = String::with_capacity(data.len() + data.len() / 2 + 200 + extra.len());
    html.push_str("<!DOCTYPE html>\n<html><head>\n<link rel=\"stylesheet\" href=\"/files/maintheme.css\"></head><body>");
    html.push_str(extra);
    let mut html_generator =
        ClassedHTMLGenerator::new_with_class_style(syntax, &ss, ClassStyle::Spaced);
    for line in LinesWithEndings::from(data) {
        html_generator
            .parse_html_for_line_which_includes_newline(line)
            .unwrap();
    }
    let html_n = html_generator.finalize();

    html.push_str("<pre class=\"code\">");
    html_n.lines().for_each(|line| {
        html.push_str("<i></i>");
        html.push_str(line);
        html.push('\n')
    });
    html.push_str("</pre>\n</body></html>");
    // let Ok(html) = highlighted_html_for_string(data, ss, syntax, theme) else {
    //     return None
    // };
    html
}
// use crate::SYNTAXSET;
// use crate::THEME;
// pub fn text_for_image<'a>(
//     data: &'a str,
//     ss: &'a SyntaxSet,
//     syntax: &'a SyntaxReference,
//     theme: &'a Theme,
// ) -> impl Iterator<Item = Vec<(Style, &'static str)>> + 'a {
//     let mut h = HighlightLines::new(syntax, &THEME);
//     //
//     let lines =
//         LinesWithEndings::from(data).filter_map(|line| h.highlight_line(line, &SYNTAXSET).ok());
//     lines
//     // .collect::<Vec<_>>();
// }
