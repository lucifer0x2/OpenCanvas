//! `import_asset` with `sourceProjectId` copies one asset, or one font with its registration,
//! from another project. Storage checks the source bytes against their manifest hash, so a
//! copy is an ordinary content-addressed import and replays like one.
use super::*;

pub(super) struct Copied {
    pub media_type: String,
    pub bytes: Vec<u8>,
    pub font: Option<FontRegistration>,
}

pub(super) async fn read(
    state: &AppState,
    project: String,
    asset: Option<String>,
    font: Option<String>,
) -> Result<Copied> {
    match (asset, font) {
        (Some(asset), None) => {
            let (asset, bytes) = state
                .project(project, move |p| p.read_asset(&asset))
                .await?;
            Ok(Copied {
                media_type: asset.media_type,
                bytes,
                font: None,
            })
        }
        (None, Some(font)) => {
            let (font, media_type, bytes) =
                state.project(project, move |p| p.read_font(&font)).await?;
            if font.asset_id.is_none() {
                return Err(ApiError::invalid(
                    "Only imported fonts can be copied; font_bundled is built in",
                ));
            }
            let registration = FontRegistration {
                id: font.id,
                family: font.family,
                weight: font.weight,
                style: font.style,
                license: font.license,
            };
            Ok(Copied {
                media_type,
                bytes,
                font: Some(registration),
            })
        }
        _ => Err(ApiError::invalid("Copy exactly one of assetId or fontId")),
    }
}
