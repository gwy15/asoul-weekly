use std::fmt::{self, Display};

pub enum Element {
    /// <p> <br> </p>
    Spacer,
    /// 图片
    Figure {
        /// b站cdn链接
        src: String,
        width: usize,
        height: usize,
        /// 图片大小（bytes）
        data_size: usize,
        /// 图片标题
        caption: String,
    },
    /// 简单图片，如分割线
    SimpleFigure { src: String, class: String },
    ///
    Text {
        /// 对应 p.style="text-align:center;"
        center: bool,
        /// 对应 <p> <strong> <span/> </strong> </p>
        strong: bool,
        /// 对应 span.class
        classes: Vec<String>,
        text: String,
    },
    ///
    BlockQuote { text: String },
    /// 站内视频连接，如果是两个， img.class="column" 否则为 "nomal"
    VideoLink {
        /// 封面 url
        cover: String,
        width: u32,
        height: u32,
        data_size: usize,
        aids: Vec<String>,
    },
    /// 偷懒的原始内容
    Raw(String),
}

impl Element {
    pub fn spacer() -> Self {
        Element::Spacer
    }
    pub fn block_quote(s: impl Into<String>) -> Self {
        Element::BlockQuote { text: s.into() }
    }
    pub fn figure(
        src: impl Into<String>,
        width: usize,
        height: usize,
        data_size: usize,
        caption: impl Into<String>,
    ) -> Self {
        Element::Figure {
            src: src.into(),
            width,
            height,
            data_size,
            caption: caption.into(),
        }
    }
    pub fn simple_figure(src: impl Into<String>, class: impl Into<String>) -> Self {
        Element::SimpleFigure {
            src: src.into(),
            class: class.into(),
        }
    }
    pub fn raw(s: impl Into<String>) -> Self {
        Element::Raw(s.into())
    }
}

impl Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Element::Spacer => f.write_str("<p><br></p>"),
            Element::Figure {
                src,
                width,
                height,
                data_size,
                caption,
            } => {
                f.write_str(r#"<figure class="img-box" contenteditable="false">"#)?;
                write!(
                    f,
                    "<img src=\"{src}\" width=\"{width}\" height=\"{height}\" data-size=\"{data_size}\">",
                    src = src.trim_start_matches("https:"),
                    width = width,
                    height = height,
                    data_size = data_size
                )?;
                if !caption.is_empty() {
                    f.write_str(r#"<figcaption class="caption" contenteditable="">"#)?;
                    f.write_str(caption)?;
                    f.write_str("</figcaption>")?;
                }
                f.write_str("</figure>")
            }
            Element::SimpleFigure { src, class } => {
                f.write_str(r#"<figure class="img-box" contenteditable="false">"#)?;
                write!(
                    f,
                    "<img src=\"{src}\" class=\"{class}\">",
                    src = src.trim_start_matches("https:"),
                    class = class
                )?;
                f.write_str("</figure>")
            }
            Element::Text {
                center,
                strong,
                classes,
                text,
            } => {
                if *center {
                    f.write_str("<p style=\"text-align: center;\">")?;
                } else {
                    f.write_str("<p>")?;
                }
                if *strong {
                    f.write_str("<strong>")?;
                }
                // span
                let styles = classes.join(" ");
                write!(f, "<span class=\"{}\">", styles)?;
                f.write_str(text)?;
                f.write_str("</span>")?;
                //
                if *strong {
                    f.write_str("</strong>")?;
                }
                f.write_str("</p>")
            }
            Element::BlockQuote { text } => {
                f.write_str("<blockquote><p>")?;
                f.write_str(text)?;
                f.write_str("</p></blockquote>")
            }
            Element::VideoLink {
                cover,
                width,
                height,
                data_size,
                aids,
            } => {
                f.write_str(r#"<figure class="img-box" contenteditable="false">"#)?;
                let class = if aids.len() == 1 { "nomal" } else { "column" };
                let aids = aids.join(",");
                let src = cover.trim_start_matches("https:");
                write!(f,
                    "<img src=\"{src}\" width=\"{width}\" height=\"{height}\" data-size=\"{data_size}\" aid=\"{aids}\" class=\"video-card {class}\" type=\"{class}\">",
                    src     = src,
                    height  = height,
                    width   = width,
                    data_size = data_size,
                    aids    = aids,
                    class   = class
                )?;
                f.write_str("</figure>")
            }
            Element::Raw(s) => f.write_str(s),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_spacer() {
        let spacer = Element::Spacer;
        assert_eq!(spacer.to_string(), "<p><br></p>");
    }

    #[test]
    fn test_figure() {
        let f = Element::Figure {
            src: "https://example.com".to_string(),
            width: 1920,
            height: 1080,
            data_size: 114514,
            caption: "标题".to_string(),
        };
        assert_eq!(f.to_string(), r#"
        <figure class="img-box" contenteditable="false"><img src="//example.com" width="1920" height="1080" data-size="114514"><figcaption class="caption" contenteditable="">标题</figcaption></figure>
        "#.trim());

        let f = Element::Figure {
            src: "https://example.com".to_string(),
            width: 1920,
            height: 1080,
            data_size: 114514,
            caption: "".to_string(),
        };
        assert_eq!(f.to_string(), r#"
        <figure class="img-box" contenteditable="false"><img src="//example.com" width="1920" height="1080" data-size="114514"></figure>
        "#.trim());
    }

    #[test]
    fn test_blockquote() {
        let b = Element::BlockQuote {
            text: "一个魂们大家好".to_string(),
        };
        assert_eq!(
            b.to_string(),
            "<blockquote><p>一个魂们大家好</p></blockquote>"
        );
    }
}
