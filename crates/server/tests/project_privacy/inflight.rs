use super::*;
use public_support::HOST;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
async fn a_public_export_in_flight_cannot_return_data_after_demotion() {
    let local = Server::start().await;
    let public = PublicServer::start(local.state.clone()).await;
    let project = local.project().await;
    assert_eq!(
        local.batch(&project, 0, "layout", support::design()).await["committedRevision"],
        1
    );
    set_privacy(&local, &project, "public").await;
    let payload = json!({
        "nodeIds":["frame"],
        "settings":[{"format":"svg"}],
        "revision":1
    })
    .to_string();
    let mut connection = BufReader::new(
        tokio::net::TcpStream::connect(public.url.strip_prefix("http://").unwrap())
            .await
            .unwrap(),
    );
    connection
        .get_mut()
        .write_all(
            format!(
                "POST {PROJECTS}/{project}/exports HTTP/1.1\r\nHost: {HOST}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nExpect: 100-continue\r\n\r\n",
                payload.len()
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    // Hyper emits this response when the export handler begins reading the JSON body,
    // after the public route has already passed its initial visibility check.
    let interim = read_line(&mut connection).await;
    assert!(interim.starts_with("HTTP/1.1 100 "), "{interim}");
    assert_eq!(read_line(&mut connection).await, "\r\n");
    set_privacy(&local, &project, "private").await;
    connection
        .get_mut()
        .write_all(payload.as_bytes())
        .await
        .unwrap();
    let final_status = read_line(&mut connection).await;
    assert!(final_status.starts_with("HTTP/1.1 404 "), "{final_status}");
}

async fn read_line(connection: &mut BufReader<tokio::net::TcpStream>) -> String {
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(5), connection.read_line(&mut line))
        .await
        .unwrap()
        .unwrap();
    line
}
