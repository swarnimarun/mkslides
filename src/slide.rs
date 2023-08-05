use std::rc::Rc;

use anyhow::Result;
use comrak::{
    arena_tree::NodeEdge,
    nodes::{AstNode, NodeValue},
};
use itertools::Itertools;
use ratatui::{
    prelude::{Alignment, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Span, Spans, Text},
    widgets::{self, Block, Borders, ListItem, Padding, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone)]
pub(crate) enum SlideItem {
    Heading(String),
    Paragraph(String),
    Bullets(Vec<String>),
    Code(String),
    QR(String),
}

impl SlideItem {
    fn render<B: ratatui::backend::Backend>(&self, frame: &mut Frame<B>, rect: Rect) -> u16 {
        match self {
            SlideItem::Heading(src) => {
                let x = Style::default()
                    .bg(Color::Black)
                    .fg(Color::White)
                    .italic()
                    .bold();
                let b = Block::default().style(x).title_alignment(Alignment::Center);
                frame.render_widget(
                    ratatui::widgets::Paragraph::new(src.as_str())
                        .block(b)
                        .alignment(Alignment::Center),
                    Rect {
                        width: src.len() as u16 + 4,
                        height: 1,
                        ..rect
                    },
                );
                2 + rect.y
            }
            SlideItem::Paragraph(src) => {
                let x = Style::default().italic();
                let b = Block::default().style(x).title_alignment(Alignment::Left);
                let lines = src.lines().map(|x| x.len());
                let max_len = lines.clone().max().unwrap_or(0);
                let lines = lines.count();
                let widget = ratatui::widgets::Paragraph::new(src.as_str())
                    .wrap(Wrap { trim: true })
                    .block(b);
                let max_width = 80;
                let height = (max_len / max_width) + 2;
                let width = if height == 1 { max_len } else { max_width };
                frame.render_widget(
                    widget,
                    Rect {
                        width: width as u16,
                        height: height as u16,
                        ..rect
                    },
                );
                lines as u16 + 2 + rect.y
            }
            SlideItem::Bullets(ls) => {
                let lines = ls.iter().map(|x| x.len());
                let max_len = lines.clone().max().unwrap_or(0);
                let lines = lines.count();
                let items = ls
                    .into_iter()
                    .map(|x| ListItem::new("- ".to_string() + x.as_str()))
                    .collect::<Vec<_>>();
                frame.render_widget(
                    widgets::List::new(items),
                    Rect {
                        width: max_len as u16 + 5,
                        height: lines as u16,
                        ..rect
                    },
                );
                lines as u16 + 2 + rect.y
            }
            SlideItem::Code(src) => {
                let text = ratatui::text::Text::raw(src.as_str());
                let width = text.width();
                let height = text.height();
                frame.render_widget(
                    ratatui::widgets::Paragraph::new(text)
                        .block(Block::new().borders(Borders::LEFT)),
                    Rect {
                        width: width as u16 + 2,
                        height: height as u16,
                        ..rect
                    },
                );
                height as u16 + 2 + rect.y
            }
            SlideItem::QR(src) => {
                let qr = qrcode::QrCode::new(src)
                    .unwrap()
                    .render()
                    .quiet_zone(false)
                    .dark_color('█')
                    .light_color(' ')
                    .build();
                let qr = qr
                    .lines()
                    .map(|x| x.chars().map(|c| [c, c]).flatten().collect::<String>())
                    .join("\n");
                let lines = qr.lines().map(|x| x.chars().count());
                let max_len = lines.clone().max().unwrap_or(0);
                let lines = lines.count();
                frame.render_widget(
                    ratatui::widgets::Paragraph::new(qr.as_str()),
                    Rect {
                        width: max_len as u16,
                        height: lines as u16,
                        ..rect
                    },
                );
                lines as u16 + 2 + rect.y
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Slide {
    title: String,
    items: Vec<SlideItem>,
}

#[derive(Debug)]
pub(crate) struct Slides {
    title: String,
    slides: Vec<Slide>,
    current_idx: usize,
}

impl Slides {
    pub(crate) fn current(&self) -> Option<&Slide> {
        self.slides.get(self.current_idx)
    }
    pub(crate) fn next(&mut self) {
        self.current_idx = (self.current_idx + 1).min(self.slides.len() - 1);
    }
    pub(crate) fn prev(&mut self) {
        self.current_idx = self.current_idx.saturating_sub(1)
    }
}

pub(crate) fn mkslides(path: impl AsRef<str>) -> Result<Slides> {
    let md_slides = std::fs::read_to_string(path.as_ref())?;
    use comrak::{parse_document, Arena};
    let arena = Arena::new();
    let slides = md_slides
        .split("---")
        .map(|x| x.trim_matches('-').trim())
        .map(|x| parse_document(&arena, x, &comrak::ComrakOptions::default()))
        .map(|node| {
            let mut items = vec![];
            let mut new = true;
            node.traverse().for_each(|node| {
                let node = match node {
                    NodeEdge::Start(node) => node,
                    NodeEdge::End(_) => {
                        new = true;
                        return;
                    }
                };
                match &node.data.borrow().value {
                    NodeValue::List(_) => {
                        // println!("## LIST");
                        items.push(SlideItem::Bullets(vec![]));
                        new = false;
                    }
                    NodeValue::Heading(_) => {
                        items.push(SlideItem::Heading("".into()));
                        new = false;
                    }
                    NodeValue::CodeBlock(codeblock) => {
                        match codeblock.info.as_str() {
                            "qrcode" => {
                                items.push(SlideItem::QR(codeblock.literal.trim().to_owned()));
                            }
                            _ => {
                                items.push(SlideItem::Code(codeblock.literal.clone()));
                            }
                        }
                        new = false;
                    }
                    NodeValue::Item(_) => {
                        // println!("## ITEM");
                        items.last_mut().map(|item| {
                            if let SlideItem::Bullets(bullets) = item {
                                bullets.push("".into());
                            }
                        });
                        new = false;
                    }
                    NodeValue::Code(code) => {
                        let src = code.literal.as_str();
                        if new {
                            items.push(SlideItem::Paragraph("".into()));
                            new = false;
                        }
                        items.last_mut().map(|item| match item {
                            SlideItem::Paragraph(psrc) | SlideItem::Heading(psrc) => {
                                psrc.push('`');
                                psrc.push_str(src);
                                psrc.push('`');
                            }
                            SlideItem::Bullets(bullets) => {
                                bullets.last_mut().map(|b| {
                                    b.push('`');
                                    b.push_str(src);
                                    b.push('`');
                                });
                            }
                            _ => {}
                        });
                    }
                    NodeValue::Text(src) => {
                        // println!("{src}");
                        if new {
                            items.push(SlideItem::Paragraph("".into()));
                            new = false;
                        }
                        items.last_mut().map(|item| match item {
                            SlideItem::Paragraph(psrc) | SlideItem::Heading(psrc) => {
                                psrc.push_str(src)
                            }
                            SlideItem::Bullets(bullets) => {
                                bullets.last_mut().map(|b| b.push_str(src));
                            }
                            _ => {}
                        });
                    }
                    _ => {}
                };
            });
            Slide {
                title: path.as_ref().into(),
                items,
            }
        })
        .collect::<Vec<_>>();
    // println!("{slides:?}");
    Ok(Slides {
        title: path.as_ref().into(),
        slides,
        current_idx: 0,
    })
}

pub(crate) fn render_slide<B: ratatui::backend::Backend>(
    slide: &Slide,
) -> Box<dyn FnOnce(&mut Frame<B>)> {
    let src = slide.title.clone();
    let items = slide.items.clone();
    Box::new(move |frame| {
        frame.render_widget(
            Block::new()
                .title(src.as_str())
                .white()
                .on_blue()
                .title_alignment(Alignment::Center),
            Rect {
                x: 0,
                y: 1,
                width: src.len() as u16 + 2,
                height: 1,
            },
        );
        // for item in items {
        if items.len() == 0 {
            return;
        }
        let mut prev_y = 4;
        for item in &items {
            prev_y = item.render(
                frame,
                Rect {
                    x: 4,
                    y: prev_y,
                    width: frame.size().width - 8,
                    height: frame.size().height - prev_y,
                },
            );
        }
    })
}
