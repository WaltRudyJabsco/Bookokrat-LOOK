use super::MarkdownTextReader;
use crate::markdown::{Block, Node, TableCellContent};

fn node_text(node: &Node) -> String {
    let join = |nodes: &[Node]| nodes.iter().map(node_text).collect::<Vec<_>>().join(" ");
    match &node.block {
        Block::Heading { content, .. } | Block::Paragraph { content } => {
            MarkdownTextReader::text_to_string(content)
        }
        Block::Quote { content } | Block::EpubBlock { content, .. } => join(content),
        Block::List { items, .. } => items
            .iter()
            .map(|item| join(&item.content))
            .collect::<Vec<_>>()
            .join(" "),
        Block::DefinitionList { items } => items
            .iter()
            .map(|item| {
                format!(
                    "{} {}",
                    MarkdownTextReader::text_to_string(&item.term),
                    item.definitions
                        .iter()
                        .map(|nodes| join(nodes))
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            })
            .collect::<Vec<_>>()
            .join(" "),
        Block::Table { header, rows, .. } => header
            .iter()
            .chain(rows.iter())
            .flat_map(|row| row.cells.iter())
            .map(|cell| match &cell.content {
                TableCellContent::Simple(text) => MarkdownTextReader::text_to_string(text),
                TableCellContent::Rich(nodes) => join(nodes),
            })
            .collect::<Vec<_>>()
            .join(" "),
        Block::CodeBlock { content, .. } => content.clone(),
        Block::ThematicBreak => String::new(),
    }
}

impl MarkdownTextReader {
    pub fn narration_passage(&self, start: usize) -> Option<(usize, String)> {
        self.markdown_document
            .as_ref()?
            .blocks
            .iter()
            .enumerate()
            .skip(start)
            .find_map(|(index, node)| {
                let text = node_text(node)
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                (!text.is_empty()).then_some((index, text))
            })
    }

    pub fn set_narration_node(&mut self, node: Option<usize>) {
        self.narration_node = node;
        self.narration_range = None;
        self.narration_follow_pending = node.is_some();
        self.refresh_narration_range();
    }

    pub(super) fn refresh_narration_range(&mut self) {
        let Some(node) = self.narration_node else {
            return;
        };
        if self.narration_range.is_some() {
            return;
        }
        if let Some(start) = self
            .rendered_content
            .lines
            .iter()
            .position(|line| line.node_index == Some(node))
        {
            let end = self
                .rendered_content
                .lines
                .iter()
                .enumerate()
                .skip(start + 1)
                .find(|(_, line)| line.node_index.is_some_and(|index| index != node))
                .map(|(index, _)| index)
                .unwrap_or(self.rendered_content.lines.len());
            self.narration_range = Some(start..end);
            if self.narration_follow_pending {
                self.scroll_to_line(start.saturating_sub(self.visible_height / 3));
                self.narration_follow_pending = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn narration_extracts_native_paragraphs_without_markup() {
        use crate::parsing::html_to_markdown::HtmlToMarkdownConverter;
        let document = HtmlToMarkdownConverter::new()
            .convert("<h1>A chapter</h1><p>Café <b>and</b> books.</p><p>Another paragraph.</p>");
        let text: Vec<_> = document.blocks.iter().map(node_text).collect();
        assert_eq!(
            text,
            vec!["A chapter", "Café and books.", "Another paragraph."]
        );
    }
}
