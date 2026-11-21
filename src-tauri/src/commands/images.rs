//! The image library's commands (FCL-5): browse the built-in pictures by look and group,
//! search them, and see them. Putting one into a graphic stays the agent's (`library` `use`),
//! since a graphic's page has to place it.
use super::AppState;
use crate::image_library::{self as library, Catalog};
use serde::Serialize;

/// The catalogue, and the look the open project's style takes.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageLibraryView {
    pub catalog: &'static Catalog,
    /// None with no project open.
    pub project_look: Option<String>,
}

/// A picture as the window shows it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagePicture {
    pub id: String,
    /// A `data:` URL.
    pub src: String,
}

/// Most pictures handed over at once.
pub const MAX_PICTURES: usize = 60;

pub fn image_library_list_impl(state: &AppState) -> ImageLibraryView {
    let style = state
        .opened_project
        .lock()
        .as_ref()
        .map(|reader| reader.document().style.clone());
    ImageLibraryView {
        catalog: library::catalog(),
        project_look: style.map(|name| {
            library::look_for(&crate::style::store::resolve(&name))
                .id
                .clone()
        }),
    }
}

/// The ids of the pictures matching `query`, best first, in `look` and the sourced set.
pub fn image_library_search_impl(query: String, look: Option<String>) -> Vec<String> {
    let looks: Vec<&str> = look.iter().map(String::as_str).collect();
    library::search(&query, look.is_some().then_some(looks.as_slice()))
        .into_iter()
        .take(library::MAX_FOUND)
        .map(|image| image.id.clone())
        .collect()
}

/// The pictures `ids` names, to show.
pub fn image_library_pictures_impl(ids: Vec<String>) -> Result<Vec<ImagePicture>, String> {
    use base64::Engine;
    if ids.len() > MAX_PICTURES {
        return Err(format!("Ask for at most {MAX_PICTURES} pictures at once"));
    }
    ids.into_iter()
        .map(|id| {
            let image = library::find(&id)?;
            let mime = if image.file.ends_with(".svg") {
                "image/svg+xml"
            } else {
                "image/webp"
            };
            Ok(ImagePicture {
                src: format!(
                    "data:{mime};base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(image.bytes())
                ),
                id: image.id.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_lists_searches_and_shows_its_pictures() {
        let host = crate::mcp::tests_support::BareHost::default();
        let view = image_library_list_impl(&host.state);
        assert!(view.project_look.is_none(), "no project open");
        assert!(view.catalog.images.len() > 300);
        let found = image_library_search_impl("robot".into(), Some("broadsheet".into()));
        assert!(!found.is_empty());
        assert!(found.len() <= library::MAX_FOUND);
        assert!(found
            .iter()
            .all(|id| id.starts_with("broadsheet/") || id.starts_with("sourced/")));
        let pictures = image_library_pictures_impl(found[..2].to_vec()).unwrap();
        assert_eq!(pictures[0].id, found[0]);
        assert!(pictures[0].src.starts_with("data:image/"));
        assert!(pictures[0].src.len() > 1000);
        let flag = view
            .catalog
            .images
            .iter()
            .find(|i| i.file.ends_with(".svg"))
            .unwrap();
        let svg = image_library_pictures_impl(vec![flag.id.clone()]).unwrap();
        assert!(svg[0].src.starts_with("data:image/svg+xml;base64,"));
        assert!(image_library_pictures_impl(vec!["no/such".into()]).is_err());
        assert!(image_library_pictures_impl(vec![found[0].clone(); MAX_PICTURES + 1]).is_err());
    }
}
