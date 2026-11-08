//! `publish`: a video's title, description, tags and thumbnail, and uploading a finished
//! export to YouTube through Zernio (PUB-1, PUB-2). For every kind of video.
use super::tools::{params, schema, text, Ctx, Output, ToolDef};
use crate::commands::publish as pc;
use crate::publish::{Package, Release};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

pub static TOOLS: [ToolDef; 1] = [ToolDef {
    name: "publish",
    description: "Any video's YouTube package (the user sees it on the Export page) and upload. \
        `target`: `video` (default) or a short's or clip's id. `op`: `read` (default); `set` \
        with `package`; `thumbnail` with `atUs` (a frame of the edit; with `file`, of that \
        export) or `image`; `keywords` with `seed` (the subject): what people search around \
        it, monthly volume and trend (Treg, under a cent), for the title's wording and tags; \
        `upload` with `file` (a finished export) and `release`, only when the user says so, \
        with the release they named; `uploads` reads their status again.",
    schema: schema::<PublishArgs>,
    read_only: false,
    run: publish,
}];

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PublishOp {
    #[default]
    Read,
    Set,
    Thumbnail,
    Upload,
    Uploads,
    Keywords,
}

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct PublishArgs {
    /// read (default), set, thumbnail, upload or uploads.
    #[serde(default)]
    pub op: PublishOp,
    /// `video` (default) or a short's id.
    #[serde(default)]
    pub target: Option<String>,
    /// set: the package.
    #[serde(default)]
    pub package: Option<Package>,
    /// thumbnail, upload: a finished export (from export_status).
    #[serde(default)]
    pub file: Option<String>,
    /// thumbnail: the frame's time in that file, microseconds.
    #[serde(default)]
    pub at_us: Option<u64>,
    /// thumbnail: an image file instead (JPG or PNG, 1280×720 or larger).
    #[serde(default)]
    pub image: Option<String>,
    /// upload: private, scheduled (with `at`) or draft.
    #[serde(default)]
    pub release: Option<Release>,
    /// keywords: the word or phrase to start from.
    #[serde(default)]
    pub seed: Option<String>,
}

fn describe(view: &pc::PackageView) -> String {
    let p = &view.package;
    let mut out = format!(
        "{}: title \"{}\" ({} chars); description {} chars; tags: {}; thumbnail: {}; AI label: {}.",
        view.target,
        p.title,
        p.title.chars().count(),
        p.description.chars().count(),
        if p.tags.is_empty() {
            "none".to_string()
        } else {
            p.tags.join(", ")
        },
        view.thumbnail_size
            .map_or("none".to_string(), |(w, h)| format!("{w}×{h}")),
        if p.ai_disclosure { "on" } else { "off" }
    );
    if let Some(problem) = &view.problem {
        out.push_str(&format!(" Not ready to upload: {problem}."));
    }
    for u in &view.uploads {
        out.push_str(&format!(
            "\nUpload {} ({:?}) {}: {}{}{}",
            u.id,
            u.release,
            u.attempted_at.format("%Y-%m-%d %H:%M"),
            u.status,
            u.visibility
                .as_deref()
                .map_or(String::new(), |v| format!(", YouTube says {v}")),
            u.studio_link().map_or(String::new(), |l| format!(", {l}"))
        ));
    }
    out
}

fn publish(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: PublishArgs = params(args)?;
    let target = args.target.unwrap_or_else(|| "video".into());
    match args.op {
        PublishOp::Read => text(describe(&pc::publish_package_impl(ctx.state, target)?)),
        PublishOp::Set => {
            let package = args.package.ok_or("Give `package`")?;
            let view = pc::publish_package_save_impl(ctx.state, target, package)?;
            text(format!("Saved. {}", describe(&view)))
        }
        PublishOp::Thumbnail => {
            let view = match (args.image, args.file) {
                (Some(image), _) => pc::publish_thumbnail_file_impl(ctx.state, target, image)?,
                (None, Some(file)) => pc::publish_thumbnail_frame_impl(
                    ctx.state,
                    target,
                    file,
                    args.at_us.ok_or("Give `atUs`, the frame's time")?,
                )?,
                (None, None) => pc::publish_thumbnail_edit_impl(
                    ctx.state,
                    target,
                    args.at_us.ok_or(
                        "Give `atUs` (a frame of the edit), `file` and `atUs`, or `image`",
                    )?,
                )?,
            };
            text(format!("Thumbnail set. {}", describe(&view)))
        }
        PublishOp::Upload => {
            let file = args.file.ok_or("Give `file`: the finished export")?;
            let release = args
                .release
                .ok_or("Give `release`: the one the user named")?;
            let upload = pc::publish_upload_impl(ctx.state, target, file, release, &|step| {
                ctx.progress(0.5, step)
            })?;
            text(format!(
                "Uploaded: {}{}{}. AI label and the ads setting can be checked in YouTube Studio.",
                upload.status,
                upload
                    .visibility
                    .as_deref()
                    .map_or(String::new(), |v| format!(", YouTube says {v}")),
                upload
                    .studio_link()
                    .map_or(String::new(), |l| format!(", {l}"))
            ))
        }
        PublishOp::Keywords => {
            let seed = args.seed.ok_or("Give `seed`")?;
            let (keywords, cost) = crate::commands::faceless::keyword_suggestions_impl(&seed)?;
            let lines = keywords
                .iter()
                .map(|k| {
                    format!(
                        "- {}: {:.0}/month{}{}",
                        k.keyword,
                        k.volume,
                        k.trend.map_or(String::new(), |t| format!(
                            ", {}{:.0}% over 3 months",
                            if t >= 1.0 { "+" } else { "" },
                            (t - 1.0) * 100.0
                        )),
                        k.difficulty
                            .map_or(String::new(), |d| format!(", difficulty {d:.0}"))
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            text(format!(
                "{lines}\n({} through Treg.)",
                crate::faceless::treg::dollars(cost)
            ))
        }
        PublishOp::Uploads => {
            let view = pc::publish_package_impl(ctx.state, target.clone())?;
            for u in view.uploads.iter().filter(|u| u.post_id.is_some()) {
                let _ = pc::publish_refresh_impl(ctx.state, u.id.clone());
            }
            text(describe(&pc::publish_package_impl(ctx.state, target)?))
        }
    }
}
