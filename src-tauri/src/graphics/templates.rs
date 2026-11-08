//! The graphics the app ships, made for tutorials. Each is a full-screen HyperFrames
//! composition with a transparent page and a card that fills the frame, so it cuts in and out
//! (it fades only when the app asks, see [`super::Entrance`]), laid out in
//! the viewport so it fits a 16:9 video and a 9:16 short alike, with its text and colours as
//! variables. The cards animate with the Web Animations API; `blank`, the start of a graphic of
//! the agent's own, with GSAP (`super::gsap`), as HyperFrames' compositions do.
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
        "One statement to remember on a card, with a label and an optional detail."
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
        "timeline-cut",
        "Timeline, gaps closed",
        6000,
        "Shows a cut: a timeline's silent gaps close up on `cue` and its length goes from \
         `before` to `after`. For cutting silence, pauses or retakes."
    ),
    template!(
        "prompt-reply",
        "Prompt and reply",
        8000,
        "Shows a prompt typed into a chat, sent on `cue`, and the reply streaming in. For what \
         the speaker asks an agent or app, in their words."
    ),
    template!(
        "count-up",
        "Number count-up",
        5000,
        "A spoken number as the hero: its digits roll up from `cue` and land on it, with a \
         label, a prefix or suffix (a currency, a unit) and a line under it."
    ),
    template!(
        "before-after",
        "Before and after",
        6000,
        "Shows a change: one card holds the before; on `cue` a line sweeps across and leaves the \
         after in the same frame. For a result, a fix, two states compared."
    ),
    template!(
        "app-click",
        "App click",
        6000,
        "Shows an action in an app sketch: the cursor moves to a named button, clicks it on \
         `cue`, and the result appears. For \"click Join\", \"press Run\"."
    ),
    template!(
        "steps-link",
        "Steps, linked",
        6000,
        "Shows a flow: up to five steps appear from `cue`, each joined to the last by a line \
         as it is reached. For \"first this, then that\": how a process goes."
    ),
    template!(
        "items-stack",
        "Items, stacked",
        6000,
        "Shows what adds up: up to five items land one on another from `cue` and build a \
         stack, with a line under it. For \"on top of that, you also get\"."
    ),
    template!(
        "headline",
        "News headline",
        6000,
        "What happened, set as a front page: a kicker, a heavy rule, the headline with one \
         word (`emphasis`) in the accent's italic, a deck and a source tag; on `cue` a rule \
         strikes under that word. For the opening and each new turn of a story."
    ),
    template!(
        "quote",
        "Quote from a source",
        6000,
        "A report's or statement's own words with who wrote them; on `cue` a marker sweeps \
         behind the words that matter (`highlight`) and a note lands beside them. For \"the \
         report says\"."
    ),
    template!(
        "verdict",
        "Claim and verdict",
        6000,
        "A claim with up to three lines of evidence (split by |); on `cue` a stamp lands \
         across it with the verdict (Confirmed, False, Unproven). For a claim the story tests."
    ),
    template!(
        "bars",
        "Comparison bars",
        6000,
        "Up to five rows (\"Label: value\" split by |) whose bars grow to scale on `cue`, one \
         row (`highlight`) in the accent, a unit beside each value. For numbers compared."
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
