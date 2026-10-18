//! The graphics the app ships, made for tutorials. Each is a full-screen HyperFrames
//! composition with a transparent page (its card fades in and out over the video), laid out in
//! the viewport so it fits a 16:9 video and a 9:16 short alike, animated with the Web
//! Animations API (no library, no network), with its text and colours as variables.
use serde::Serialize;

pub struct Template {
    pub name: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// How long it plays when placed.
    pub length_us: u64,
    /// Files copied into the graphic's folder.
    pub files: &'static [(&'static str, &'static str)],
}

impl Template {
    pub fn html(&self) -> &'static str {
        self.files
            .iter()
            .find(|(path, _)| *path == super::ENTRY)
            .map_or("", |(_, html)| html)
    }
}

macro_rules! template {
    ($name:literal, $label:literal, $length_ms:literal, $description:literal) => {
        Template {
            name: $name,
            label: $label,
            description: $description,
            length_us: $length_ms * 1000,
            files: &[(
                "index.html",
                include_str!(concat!("../../graphics/templates/", $name, "/index.html")),
            )],
        }
    };
}

static TEMPLATES: &[Template] = &[
    template!(
        "title",
        "Section title",
        4000,
        "A chapter or section opener: a small line above, the title, a subtitle."
    ),
    template!(
        "step",
        "Step card",
        4000,
        "\"Step 3 of 5\" with a big number, the step's title and a detail line."
    ),
    template!(
        "shortcut",
        "Keyboard shortcut",
        4000,
        "Keycaps for a shortcut (Ctrl+Shift+P) that press down, with what it does."
    ),
    template!(
        "key-point",
        "Key point",
        5000,
        "One statement to remember, with an accent rule and an optional detail."
    ),
    template!(
        "recap",
        "Recap checklist",
        6000,
        "A title and up to six items ticked off one by one."
    ),
    template!(
        "end-card",
        "End card",
        6000,
        "Thanks, a call to action, the channel and what's next. Holds to the end."
    ),
    template!(
        "blank",
        "Blank",
        4000,
        "An empty composition to write from scratch (for agents and hand editing)."
    ),
];

pub fn all() -> &'static [Template] {
    TEMPLATES
}

pub fn find(name: &str) -> Result<&'static Template, String> {
    TEMPLATES.iter().find(|t| t.name == name).ok_or_else(|| {
        format!(
            "There is no graphic template \"{name}\" (there are: {})",
            TEMPLATES
                .iter()
                .map(|t| t.name)
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

/// What the Graphics tab and agents see of a template.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateView {
    pub name: String,
    pub label: String,
    pub description: String,
    pub length_us: u64,
    pub variables: Vec<super::VariableDecl>,
}

pub fn views() -> Vec<TemplateView> {
    TEMPLATES
        .iter()
        .map(|t| TemplateView {
            name: t.name.into(),
            label: t.label.into(),
            description: t.description.into(),
            length_us: t.length_us,
            variables: super::parse_declarations(t.html()).unwrap_or_default(),
        })
        .collect()
}
