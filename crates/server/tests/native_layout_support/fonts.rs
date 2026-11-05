//! Uploaded fixtures are real store assets, with pinned original bytes.
use super::{workspace, Server};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;
use sha2::{Digest, Sha256};

pub async fn upload(server: &Server, project: &str) -> u64 {
    let inter = workspace().join("spikes/native-render-parity/out/fonts/inter.ttf");
    let inter = std::fs::read(&inter).unwrap_or_else(|error| {
        panic!("Required G1 Inter variable fixture unavailable ({error}); run the documented oracle capture first, never skip this acceptance fixture")
    });
    let faces = [
        (
            "inter",
            400,
            "normal",
            inter.as_slice(),
            "4989b125924991b90d05b2d16e0e388c48f7d5bb8b30539bbf9c755278d0ccaf",
        ),
        (
            "lato-regular",
            400,
            "normal",
            include_bytes!("../../../../fixtures/fonts/Lato-Regular.ttf").as_slice(),
            "d636e4683231f931eda222d588e944d082bfd3bdba02f928bee461c0f185b251",
        ),
        (
            "lato-bold",
            700,
            "normal",
            include_bytes!("../../../../fixtures/fonts/Lato-Bold.ttf").as_slice(),
            "8a0aace75d33794eece4b28187bfc1df0bbd2888b5d8a56e01788c8d65d16be1",
        ),
        (
            "lato-italic",
            400,
            "italic",
            include_bytes!("../../../../fixtures/fonts/Lato-Italic.ttf").as_slice(),
            "e399c44efe1387100531d26c7e4800c5d12251b890d6654a3098c7c679cb1786",
        ),
    ];
    for (revision, (id, weight, style, bytes, hash)) in faces.into_iter().enumerate() {
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), hash, "font {id}");
        let result = server
            .call(
                "import_asset",
                json!({"projectId":project,
            "expectedRevision":revision,"idempotencyKey":format!("layout-font-{id}"),
            "mediaType":"font/ttf","base64":STANDARD.encode(bytes),
            "font":{"id":id,"family":format!("Layout parity {id}"),"weight":weight,
                "style":style,"license":"OFL-1.1"}}),
            )
            .await;
        assert_eq!(result["committedRevision"], revision + 1, "{result}");
        eprintln!(
            "FONT {id} weight={weight} style={style} bytes={} sha256={hash}",
            bytes.len()
        );
    }
    4
}
