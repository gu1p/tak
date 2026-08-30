use tokio::io::{AsyncWriteExt, BufReader};

#[tokio::test]
async fn local_protocol_frame_reader_rejects_input_before_growing_past_its_bound() {
    let (reader, mut writer) = tokio::io::duplex(32);
    writer.write_all(b"123456789\n").await.unwrap();
    drop(writer);
    let mut reader = BufReader::new(reader);

    let error = super::read_frame(&mut reader, 8).await.unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}

#[tokio::test]
async fn local_protocol_frame_reader_preserves_one_complete_frame_at_a_time() {
    let (reader, mut writer) = tokio::io::duplex(32);
    writer.write_all(b"one\ntwo\n").await.unwrap();
    drop(writer);
    let mut reader = BufReader::new(reader);

    assert_eq!(
        super::read_frame(&mut reader, 8).await.unwrap(),
        Some("one\n".into())
    );
    assert_eq!(
        super::read_frame(&mut reader, 8).await.unwrap(),
        Some("two\n".into())
    );
    assert_eq!(super::read_frame(&mut reader, 8).await.unwrap(), None);
}
