use super::*;

#[test]
fn client_id_decoding() {
    let client_id_str = "z6MkodHZwneVRShtaLf8JKYkxpDGp1vGZnpGmdBpX8M2exxH";
    let client_id_bin = client_id_str.parse::<DecodedClientId>().unwrap();

    assert_eq!(client_id_str, ClientId::from(client_id_bin).as_ref());

    assert!(matches!(
        "z6MkodHZwne".parse::<DecodedClientId>(),
        Err(ClientIdDecodingError::Length)
    ));
}

#[test]
fn topic_decoding() {
    let topic_str = "85089843cebc89ce5bbffd55377b2e65c8a32c2d0a76742f2d6852b5f531a460";
    let topic_bin = topic_str.parse::<DecodedTopic>().unwrap();

    assert_eq!(topic_str, Topic::from(topic_bin).as_ref());

    assert!(matches!(
        "85089843ce".parse::<DecodedTopic>(),
        Err(DecodingError::Length)
    ));
}
